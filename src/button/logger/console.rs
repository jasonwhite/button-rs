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

use std::io::{self, Write};
use std::time::Instant;

use task;

use super::traits::{Error, EventLogger, LogResult, TaskLogger};

pub struct ConsoleTask {
    start_time: Instant,
    buf: Vec<u8>,
}

impl ConsoleTask {
    pub fn new(thread: usize, task: &task::Any) -> Result<ConsoleTask, Error> {
        let mut buf = Vec::new();

        writeln!(&mut buf, "[{}] {}", thread, task)?;

        Ok(ConsoleTask {
            start_time: Instant::now(),
            buf,
        })
    }
}

impl io::Write for ConsoleTask {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buf.write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TaskLogger for ConsoleTask {
    fn finish(self, result: &Result<(), Error>) -> LogResult {
        let duration = self.start_time.elapsed();

        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        // TODO: Convert \r\n to \n on Windows.
        // TODO: Filter out ASCII escape codes if coloring is turned off.
        stdout.write_all(&self.buf)?;

        // Add a new line to the end if there isn't one.
        if !self.buf.ends_with(b"\n") {
            stdout.write_all(b"\n")?;
        }

        writeln!(stdout, "Task duration: {:.4?}", duration)?;

        if let Err(err) = result {
            // Print out the chain of errors.
            let mut errors = err.iter_chain();

            if let Some(err) = errors.next() {
                writeln!(stdout, "    Error: {}", err)?;
            }

            for err in errors {
                writeln!(stdout, "Caused by: {}", err)?;
            }
        }

        Ok(())
    }
}

/// Log text to the console.
///
/// This buffers task output and prints out the task once it has finished.
pub struct Console {
    /// Time since the build started.
    start_time: Instant,
}

impl Default for Console {
    fn default() -> Console {
        Console {
            start_time: Instant::now(),
        }
    }
}

impl Console {
    pub fn new() -> Console {
        Console::default()
    }
}

impl EventLogger for Console {
    type TaskLogger = ConsoleTask;

    fn begin(&mut self, _threads: usize) -> LogResult {
        self.start_time = Instant::now();

        Ok(())
    }

    fn end(&mut self, _result: &Result<(), Error>) -> LogResult {
        println!("Build duration: {:.4?}", self.start_time.elapsed());

        Ok(())
    }

    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<ConsoleTask, Error> {
        ConsoleTask::new(thread, task)
    }
}
