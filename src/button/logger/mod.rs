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
use std::io;
use std::ops;
use std::path::Path;

mod binary;
mod console;
pub mod events;
mod traits;

pub use self::binary::{Binary, BinaryTask};
pub use self::console::{Console, ConsoleTask};
pub use self::events::{log_from_path, log_from_reader, LogEvent};
pub use self::traits::{EventLogger, LogResult, TaskLogger};

use detect::Detected;
use error::{Error, ResultExt};
use res;
use task;

// TODO: Add additional loggers:
//
//  - web
//
//    A logger that sends all of the events to a web page for display. This
//    could be useful for seeing the output for long running tasks as they
//    occur. It could also display a Gantt chart of the build tasks.

/// A logger for writing to a file.
///
/// A stream of events (with timestamps) are written to the file. These can then
/// be read back later to "replay" the original build log through a text logger.
pub type BinaryFile = Binary<::std::io::BufWriter<::std::fs::File>>;
pub type BinaryFileTask = BinaryTask<::std::io::BufWriter<::std::fs::File>>;

/// Creates a binary file logger from a path.
pub fn binary_file_logger<P>(path: P) -> Result<BinaryFile, Error>
where
    P: AsRef<Path>,
{
    let f = fs::File::create(path.as_ref()).with_context(|_| {
        format!("Failed creating file {:?}", path.as_ref())
    })?;
    Ok(BinaryFile::from_writer(io::BufWriter::new(f))
        .with_context(|_| format!("Failed writing to {:?}", path.as_ref()))?)
}

/// Types of loggers. Useful for static dispatch of multiple loggers.
pub enum Any {
    Console(Console),
    BinaryFile(BinaryFile),
}

impl From<Console> for Any {
    fn from(l: Console) -> Self {
        Any::Console(l)
    }
}

impl From<BinaryFile> for Any {
    fn from(l: BinaryFile) -> Self {
        Any::BinaryFile(l)
    }
}

pub enum AnyTaskLogger {
    Console(ConsoleTask),
    BinaryFile(BinaryFileTask),
}

impl From<ConsoleTask> for AnyTaskLogger {
    fn from(l: ConsoleTask) -> Self {
        AnyTaskLogger::Console(l)
    }
}

impl From<BinaryFileTask> for AnyTaskLogger {
    fn from(l: BinaryFileTask) -> Self {
        AnyTaskLogger::BinaryFile(l)
    }
}

impl io::Write for AnyTaskLogger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            AnyTaskLogger::Console(l) => l.write(buf),
            AnyTaskLogger::BinaryFile(l) => l.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            AnyTaskLogger::Console(l) => l.flush(),
            AnyTaskLogger::BinaryFile(l) => l.flush(),
        }
    }
}

impl TaskLogger for AnyTaskLogger {
    fn finish(self, result: &Result<Detected, Error>) -> LogResult<()> {
        match self {
            AnyTaskLogger::Console(l) => l.finish(result),
            AnyTaskLogger::BinaryFile(l) => l.finish(result),
        }
    }
}

impl EventLogger for Any {
    type TaskLogger = AnyTaskLogger;

    fn begin_build(&mut self, threads: usize) -> LogResult<()> {
        match self {
            Any::Console(l) => l.begin_build(threads),
            Any::BinaryFile(l) => l.begin_build(threads),
        }
    }

    fn end_build(&mut self, result: &Result<(), Error>) -> LogResult<()> {
        match self {
            Any::Console(l) => l.end_build(result),
            Any::BinaryFile(l) => l.end_build(result),
        }
    }

    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<Self::TaskLogger, Error> {
        match self {
            Any::Console(l) => Ok(l.start_task(thread, task)?.into()),
            Any::BinaryFile(l) => Ok(l.start_task(thread, task)?.into()),
        }
    }

    fn delete(&self, thread: usize, resource: &res::Any) -> LogResult<()> {
        match self {
            Any::Console(l) => l.delete(thread, resource),
            Any::BinaryFile(l) => l.delete(thread, resource),
        }
    }
}

/// A list of task loggers.
pub struct TaskLoggerList<T> {
    inner: Vec<T>,
}

impl<T> TaskLoggerList<T> {
    pub fn with_capacity(capacity: usize) -> TaskLoggerList<T> {
        TaskLoggerList {
            inner: Vec::with_capacity(capacity),
        }
    }
}

impl<T> ops::Deref for TaskLoggerList<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> ops::DerefMut for TaskLoggerList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> io::Write for TaskLoggerList<T>
where
    T: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for logger in &mut self.inner {
            logger.write_all(buf)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        for logger in &mut self.inner {
            logger.flush()?;
        }

        Ok(())
    }
}

impl<T> TaskLogger for TaskLoggerList<T>
where
    T: TaskLogger,
{
    fn finish(self, result: &Result<Detected, Error>) -> LogResult<()> {
        for logger in self.inner {
            logger.finish(result)?;
        }

        Ok(())
    }
}

/// A list of loggers.
pub struct List<T> {
    inner: Vec<T>,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        List { inner: Vec::new() }
    }
}

impl<T> List<T> {
    pub fn new() -> List<T> {
        List::default()
    }
}

impl<T> ops::Deref for List<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> ops::DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> EventLogger for List<T>
where
    T: EventLogger,
{
    type TaskLogger = TaskLoggerList<T::TaskLogger>;

    fn begin_build(&mut self, threads: usize) -> LogResult<()> {
        for logger in &mut self.inner {
            logger.begin_build(threads)?;
        }

        Ok(())
    }

    fn end_build(&mut self, result: &Result<(), Error>) -> LogResult<()> {
        for logger in &mut self.inner {
            logger.end_build(result)?;
        }

        Ok(())
    }

    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<Self::TaskLogger, Error> {
        let mut list = TaskLoggerList::with_capacity(self.len());

        for logger in &self.inner {
            list.push(logger.start_task(thread, task)?);
        }

        Ok(list)
    }

    fn delete(&self, thread: usize, resource: &res::Any) -> LogResult<()> {
        for logger in &self.inner {
            logger.delete(thread, resource)?;
        }

        Ok(())
    }
}
