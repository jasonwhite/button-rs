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
use std::sync::Arc;
use std::time::Instant;

use res;
use task;

use error::Error;

use super::traits::{EventLogger, LogResult, TaskLogger};

use atty;
use termcolor as tc;
use termcolor::WriteColor;

pub struct ConsoleTask {
    verbose: bool,
    bufwriter: Arc<tc::BufferWriter>,
    buf: tc::Buffer,
    start_time: Instant,
}

impl ConsoleTask {
    pub fn new(
        verbose: bool,
        thread: usize,
        task: &task::Any,
        bufwriter: Arc<tc::BufferWriter>,
    ) -> Result<ConsoleTask, Error> {
        let mut buf = bufwriter.buffer();

        buf.set_color(
            tc::ColorSpec::new()
                .set_fg(Some(tc::Color::Green))
                .set_bold(true),
        )?;
        write!(&mut buf, "[{}] {}", thread, task)?;
        buf.reset()?;
        buf.write_all(b"\n")?;

        Ok(ConsoleTask {
            verbose,
            bufwriter,
            buf,
            start_time: Instant::now(),
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
    fn finish(self, result: &Result<(), Error>) -> LogResult<()> {
        let ConsoleTask {
            verbose,
            bufwriter,
            mut buf,
            start_time,
        } = self;

        let duration = start_time.elapsed();

        // TODO: Convert \r\n to \n on Windows.
        // TODO: Convert ASCII escape codes to Windows if necessary.

        // Add a new line to the end if there isn't one.
        if !buf.as_slice().ends_with(b"\n") {
            buf.write_all(b"\n")?;
        }

        if verbose {
            buf.set_color(tc::ColorSpec::new().set_fg(Some(tc::Color::Blue)))?;
            write!(&mut buf, "Task duration")?;
            buf.reset()?;
            writeln!(&mut buf, ": {:.4?}", duration)?;
        }

        if let Err(err) = result {
            let mut red_fg = tc::ColorSpec::new();
            red_fg.set_fg(Some(tc::Color::Red));

            // Print out the chain of errors.
            let mut errors = err.iter_chain();

            if let Some(err) = errors.next() {
                buf.set_color(&red_fg)?;
                write!(&mut buf, "    Error")?;
                buf.reset()?;
                writeln!(&mut buf, ": {}", err)?;
            }

            for err in errors {
                buf.set_color(&red_fg)?;
                write!(&mut buf, "Caused by")?;
                buf.reset()?;
                writeln!(&mut buf, ": {}", err)?;
            }
        }

        bufwriter.print(&buf)?;

        Ok(())
    }
}

/// Log text to the console.
///
/// This buffers task output and prints out the task once it has finished.
pub struct Console {
    verbose: bool,
    start_time: Instant,
    bufwriter: Arc<tc::BufferWriter>,
}

impl Console {
    pub fn new(verbose: bool, color: tc::ColorChoice) -> Console {
        // Don't use colors if stdout is piped to a file.
        let color = if color == tc::ColorChoice::Auto
            && !atty::is(atty::Stream::Stdout)
        {
            tc::ColorChoice::Never
        } else {
            color
        };

        Console {
            verbose,
            bufwriter: Arc::new(tc::BufferWriter::stdout(color)),
            start_time: Instant::now(),
        }
    }
}

impl EventLogger for Console {
    type TaskLogger = ConsoleTask;

    fn begin_build(&mut self, _threads: usize) -> LogResult<()> {
        self.start_time = Instant::now();

        Ok(())
    }

    fn end_build(&mut self, _result: &Result<(), Error>) -> LogResult<()> {
        println!("Build duration: {:.4?}", self.start_time.elapsed());

        Ok(())
    }

    fn start_task(
        &self,
        thread: usize,
        task: &task::Any,
    ) -> Result<ConsoleTask, Error> {
        ConsoleTask::new(self.verbose, thread, task, self.bufwriter.clone())
    }

    fn delete(&self, thread: usize, resource: &res::Any) -> LogResult<()> {
        println!("[{}] Deleting {}", thread, resource);

        Ok(())
    }
}
