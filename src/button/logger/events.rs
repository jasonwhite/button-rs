// Copyright (c) 2018 Jason White
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::fs;
use std::io::{self, Write};
use std::mem;
use std::path::Path;

use bincode;
use chrono::{DateTime, Utc};
use error::SerError;

use res;
use task;

use super::traits::{EventLogger, LogResult, TaskLogger};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeginBuild {
    pub threads: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EndBuild {
    pub result: Result<(), SerError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartTask {
    pub thread: usize,
    pub task: task::Any,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WriteTask {
    pub thread: usize,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FinishTask {
    pub thread: usize,
    pub result: Result<(), SerError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delete {
    pub thread: usize,
    pub resource: res::Any,
}

/// A logging event.
#[derive(Serialize, Deserialize, Debug)]
pub enum LogEvent {
    BeginBuild(BeginBuild),
    EndBuild(EndBuild),

    /// A task is started.
    StartTask(StartTask),

    /// A task receives data.
    WriteTask(WriteTask),

    /// A task is finished.
    FinishTask(FinishTask),

    /// A task is deleted.
    Delete(Delete),
}

impl From<BeginBuild> for LogEvent {
    fn from(event: BeginBuild) -> LogEvent {
        LogEvent::BeginBuild(event)
    }
}

impl From<EndBuild> for LogEvent {
    fn from(event: EndBuild) -> LogEvent {
        LogEvent::EndBuild(event)
    }
}

impl From<StartTask> for LogEvent {
    fn from(event: StartTask) -> LogEvent {
        LogEvent::StartTask(event)
    }
}

impl From<WriteTask> for LogEvent {
    fn from(event: WriteTask) -> LogEvent {
        LogEvent::WriteTask(event)
    }
}

impl From<FinishTask> for LogEvent {
    fn from(event: FinishTask) -> LogEvent {
        LogEvent::FinishTask(event)
    }
}

impl From<Delete> for LogEvent {
    fn from(event: Delete) -> LogEvent {
        LogEvent::Delete(event)
    }
}

/// Logs a stream of events from a file path.
pub fn log_from_path<P, L>(path: P, logger: L, realtime: bool) -> LogResult<()>
where
    P: AsRef<Path>,
    L: EventLogger,
{
    let f = fs::File::open(path.as_ref())?;
    log_from_reader(io::BufReader::new(f), logger, realtime)
}

/// Logs a stream of events from a reader.
pub fn log_from_reader<R, L>(
    mut reader: R,
    mut logger: L,
    realtime: bool,
) -> LogResult<()>
where
    R: io::Read,
    L: EventLogger,
{
    use std::thread::sleep;

    // Vector to hold task events for each thread.
    let mut threads: Vec<Option<L::TaskLogger>> = Vec::new();

    // The timestamp is always serialized first.
    let mut dt: DateTime<Utc> = bincode::deserialize_from(&mut reader)?;

    while let Ok((datetime, event)) =
        bincode::deserialize_from::<_, (DateTime<Utc>, _)>(&mut reader)
    {
        if realtime {
            sleep(datetime.signed_duration_since(dt).to_std()?);
        }

        dt = datetime;

        match event {
            LogEvent::BeginBuild(e) => {
                logger.begin_build(e.threads)?;
                threads = Vec::with_capacity(e.threads);
                for _ in 0..e.threads {
                    threads.push(None);
                }
            }

            LogEvent::EndBuild(e) => {
                let result = e.result.map_err(|e| e.into());
                logger.end_build(&result)?;
            }

            LogEvent::StartTask(e) => {
                threads[e.thread] = Some(logger.start_task(e.thread, &e.task)?);
            }

            LogEvent::WriteTask(e) => {
                if let Some(ref mut t) = threads[e.thread] {
                    t.write_all(&e.data)?;
                }
            }

            LogEvent::FinishTask(e) => {
                if let Some(t) = mem::replace(&mut threads[e.thread], None) {
                    let result = e.result.map_err(|e| e.into());
                    t.finish(&result)?;
                }
            }

            LogEvent::Delete(e) => {
                logger.delete(e.thread, &e.resource)?;
            }
        }
    }

    Ok(())
}
