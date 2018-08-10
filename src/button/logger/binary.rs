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

use std::error::Error as ErrorTrait;
use std::io;
use std::sync::{Arc, Mutex};

use super::traits::{EventLogger, LogResult, TaskLogger};

use res;
use task;

use bincode;
use chrono::Utc;
use error::Error;

use super::events::*;

pub struct BinaryTask<W> {
    thread: usize,
    writer: Arc<Mutex<BinaryWriter<W>>>,
}

impl<W> io::Write for BinaryTask<W>
where
    W: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut writer = self.writer.lock().unwrap();
        match writer.dump(
            WriteTask {
                datetime: Utc::now(),
                thread: self.thread,
                data: buf.to_vec(),
            }.into(),
        ) {
            Ok(()) => Ok(buf.len()),
            Err(err) => {
                Err(io::Error::new(io::ErrorKind::Other, err.description()))
            }
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf)?;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W> TaskLogger for BinaryTask<W>
where
    W: io::Write,
{
    fn finish(self, result: &LogResult<()>) -> LogResult<()> {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => {
                Err(err.iter_chain().map(|c| format!("{}", c)).collect())
            }
        };

        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            FinishTask {
                datetime: Utc::now(),
                thread: self.thread,
                result,
            }.into(),
        )?;
        Ok(())
    }
}

struct BinaryWriter<W> {
    writer: W,
}

impl<W> BinaryWriter<W>
where
    W: io::Write,
{
    /// Serializes a log event.
    pub fn dump(&mut self, event: LogEvent) -> bincode::Result<()> {
        bincode::serialize_into(&mut self.writer, &event)
    }
}

pub struct Binary<W> {
    writer: Arc<Mutex<BinaryWriter<W>>>,
}

impl<W> Binary<W> {
    pub fn from_writer(writer: W) -> Binary<W> {
        Binary {
            writer: Arc::new(Mutex::new(BinaryWriter { writer })),
        }
    }
}

impl<W> EventLogger for Binary<W>
where
    W: io::Write + Send,
{
    type TaskLogger = BinaryTask<W>;

    fn begin_build(&mut self, threads: usize) -> LogResult<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            BeginBuild {
                datetime: Utc::now(),
                threads,
            }.into(),
        )?;
        Ok(())
    }

    fn end_build(&mut self, result: &Result<(), Error>) -> LogResult<()> {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => {
                Err(err.iter_chain().map(|c| format!("{}", c)).collect())
            }
        };

        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            EndBuild {
                datetime: Utc::now(),
                result,
            }.into(),
        )?;
        Ok(())
    }

    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<Self::TaskLogger, Error> {
        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            StartTask {
                datetime: Utc::now(),
                thread,
                task: task.clone(),
            }.into(),
        )?;

        Ok(BinaryTask {
            thread,
            writer: self.writer.clone(),
        })
    }

    fn delete(&self, thread: usize, resource: &res::Any) -> LogResult<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            Delete {
                datetime: Utc::now(),
                thread,
                resource: resource.clone(),
            }.into(),
        )?;

        Ok(())
    }
}

/// Iterator over events in the serialized binary log.
pub struct EventStream<R> {
    reader: R,
}

impl<R> EventStream<R>
where
    R: io::Read,
{
    pub fn new(reader: R) -> EventStream<R> {
        EventStream { reader }
    }
}

impl<R> Iterator for EventStream<R>
where
    R: io::Read,
{
    type Item = LogEvent;

    fn next(&mut self) -> Option<Self::Item> {
        match bincode::deserialize_from(&mut self.reader) {
            Ok(event) => Some(event),
            Err(_) => None, // TODO: Handle this error somehow
        }
    }
}
