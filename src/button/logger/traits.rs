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

use std::io;

use detect::Detected;
use error::{BuildError, Error};

use res;
use task;

/// A log result represents the result of the logging operation itself. For
/// example, events should use this to propagate errors relating to IO.
pub type LogResult<T> = Result<T, Error>;

pub trait TaskLogger: io::Write {
    /// Finishes the task.
    fn finish(self, result: &Result<Detected, Error>) -> LogResult<()>;
}

/// An event logger.
///
/// Build events get sent to the logger and the logger decides how to display
/// them.
pub trait EventLogger: Send + Sync {
    type TaskLogger: TaskLogger;

    /// Called when the build has started.
    fn begin_build(&mut self, threads: usize) -> LogResult<()>;

    /// Called when the build has finished.
    fn end_build(&mut self, result: &Result<(), BuildError>) -> LogResult<()>;

    /// Called when a task is about to be executed.
    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<Self::TaskLogger, Error>;

    /// Called when a resource is deleted.
    fn delete(
        &self,
        thread: usize,
        resource: &res::Any,
        result: &Result<(), Error>,
    ) -> LogResult<()>;
}
