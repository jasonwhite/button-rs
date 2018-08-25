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
use error::{Error, SerError};

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

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W> TaskLogger for BinaryTask<W>
where
    W: io::Write,
{
    fn finish(self, result: &LogResult<task::Detected>) -> LogResult<()> {
        let result = match result {
            Ok(x) => Ok(x.clone()),
            Err(err) => Err(SerError::new(err)),
        };

        let mut writer = self.writer.lock().unwrap();
        writer.dump(
            FinishTask {
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
    pub fn new(mut writer: W) -> Result<BinaryWriter<W>, bincode::Error> {
        // Always serialize the date time for when the logger is constructed.
        // This is used to calculate durations for all subsequent events that
        // are written.
        let dt = Utc::now();
        bincode::serialize_into(&mut writer, &dt)?;

        Ok(BinaryWriter { writer })
    }

    /// Serializes a log event.
    pub fn dump(&mut self, event: LogEvent) -> Result<(), bincode::Error> {
        // Always serialize a timestamp with the event.
        let event = (Utc::now(), event);

        bincode::serialize_into(&mut self.writer, &event)
    }
}

pub struct Binary<W> {
    writer: Arc<Mutex<BinaryWriter<W>>>,
}

impl<W> Binary<W>
where
    W: io::Write,
{
    pub fn from_writer(writer: W) -> Result<Binary<W>, Error> {
        Ok(Binary {
            writer: Arc::new(Mutex::new(BinaryWriter::new(writer)?)),
        })
    }
}

impl<W> EventLogger for Binary<W>
where
    W: io::Write + Send,
{
    type TaskLogger = BinaryTask<W>;

    fn begin_build(&mut self, threads: usize) -> LogResult<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.dump(BeginBuild { threads }.into())?;
        Ok(())
    }

    fn end_build(&mut self, result: &Result<(), Error>) -> LogResult<()> {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => Err(SerError::new(err)),
        };

        let mut writer = self.writer.lock().unwrap();
        writer.dump(EndBuild { result }.into())?;
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
                thread,
                resource: resource.clone(),
            }.into(),
        )?;

        Ok(())
    }
}
