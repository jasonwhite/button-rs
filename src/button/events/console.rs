// Copyright (c) 2019 Jason White
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
use std::thread::{self, JoinHandle};
use std::time::Duration;

use console::style;
use humantime::format_duration;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::{
    BeginTaskEvent, ChecksumErrorEvent, DeleteEvent, EndBuildEvent,
    EndTaskEvent, Event, EventHandler, Timestamp,
};

#[derive(Clone)]
struct TaskState {
    /// Time the task started.
    start: Timestamp,

    /// Progress bar associated with this task.
    pb: ProgressBar,

    /// String of the task being executed.
    name: String,

    /// Buffer of output for the task.
    buf: Vec<u8>,
}

impl TaskState {
    pub fn new(start: Timestamp, pb: ProgressBar) -> Self {
        TaskState {
            start,
            pb,
            name: String::new(),
            buf: Vec::new(),
        }
    }
}

/// Calculates the number of spaces a number takes up. Useful for padding
/// decimal numbers.
fn num_width(mut max_value: usize) -> usize {
    let mut count = 0;

    while max_value > 0 {
        max_value /= 10;
        count += 1;
    }

    count
}

/// "Inner" that lives as long as a single build. This is created and destroyed
/// for `BeginBuildEvent`s and `EndBuildEvent`s respectively.
struct Inner {
    // Vector of in-flight tasks.
    tasks: Vec<TaskState>,

    // Time at which the build started. This is used to calculate the duration
    // of the build when it finishes.
    start_time: Timestamp,

    // Progress bar thread.
    pb_thread: JoinHandle<Result<(), io::Error>>,

    // Continuously updates each of the progress bars.
    tick_thread: JoinHandle<()>,

    // Name of the build.
    name: String,
}

impl Inner {
    pub fn new(threads: usize, name: String, timestamp: Timestamp) -> Self {
        // Give a bogus start time. This will be changed as we receive events.
        let mut tasks = Vec::with_capacity(threads);
        let mut bars = Vec::with_capacity(threads);

        let progress = MultiProgress::new();

        for i in 0..threads {
            let pb = progress.add(ProgressBar::new_spinner());
            pb.set_style(Console::style_idle());
            pb.set_prefix(&format!(
                "[{:>width$}]",
                i + 1,
                width = num_width(threads)
            ));
            pb.set_message(&style("Idle").dim().to_string());

            // Clone the progress bar handle so we can update them later.
            bars.push(pb.clone());

            tasks.push(TaskState::new(timestamp, pb));
        }

        let pb_thread = thread::spawn(move || progress.join_and_clear());
        let tick_thread = thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(200));

            for pb in &bars {
                if pb.is_finished() {
                    return;
                }

                pb.tick();
            }
        });

        Inner {
            tasks,
            start_time: timestamp,
            pb_thread,
            tick_thread,
            name,
        }
    }

    pub fn finish(mut self) -> Result<(), io::Error> {
        for task in self.tasks.iter_mut() {
            task.pb
                .finish_with_message(&style("Done").dim().to_string());
        }

        self.tick_thread.join().unwrap();
        self.pb_thread.join().unwrap()?;

        Ok(())
    }

    pub fn end_build(
        self,
        timestamp: Timestamp,
        event: EndBuildEvent,
    ) -> Result<(), io::Error> {
        let duration = (timestamp - self.start_time).to_std().unwrap();
        let duration = format_duration(duration);

        let msg = match event.result {
            Ok(()) => format!(
                "{} {} in {}",
                style("Finished").bold().green(),
                style(&self.name).yellow(),
                style(duration).cyan(),
            ),
            Err(err) => format!(
                "{} {} after {}: {}",
                style("Failed").bold().red(),
                style(&self.name).yellow(),
                style(duration).cyan(),
                err
            ),
        };

        for task in &self.tasks {
            task.pb.set_style(Console::style_idle());
        }

        self.tasks[0].pb.println(&msg);

        self.finish()
    }

    pub fn begin_task(
        &mut self,
        timestamp: Timestamp,
        event: BeginTaskEvent,
    ) -> Result<(), io::Error> {
        let mut task = &mut self.tasks[event.id];
        task.start = timestamp;

        let name = event.task.to_string();

        task.pb.reset_elapsed();
        task.pb.set_style(Console::style_running());
        task.pb.set_message(&name);

        task.name = name;

        Ok(())
    }

    pub fn end_task(
        &mut self,
        timestamp: Timestamp,
        event: EndTaskEvent,
    ) -> Result<(), io::Error> {
        let task = &mut self.tasks[event.id];

        let duration = (timestamp - task.start).to_std().unwrap();
        let duration = format_duration(duration);

        if let Err(err) = event.result {
            writeln!(
                &mut task.buf,
                "{} after {}: {}",
                style("Task failed").bold().red(),
                style(duration).cyan(),
                style(err).red(),
            )?;

            task.pb.println(format!(
                "> {}\n{}",
                style(&task.name).bold().red(),
                String::from_utf8_lossy(&task.buf),
            ));
        }

        task.buf.clear();

        task.pb.set_style(Console::style_idle());

        Ok(())
    }

    pub fn delete(
        &mut self,
        _timestamp: Timestamp,
        event: DeleteEvent,
    ) -> Result<(), io::Error> {
        let task = &mut self.tasks[event.id];
        task.pb.set_style(Console::style_running());
        task.pb.set_message(&format!("Deleted {}", event.resource));

        if let Err(err) = event.result {
            task.pb.println(format!(
                "{} to delete `{}`: {}",
                style("Failed").bold().red(),
                style(event.resource).yellow(),
                err
            ));
        }

        Ok(())
    }

    pub fn checksum_error(
        &mut self,
        _timestamp: Timestamp,
        event: ChecksumErrorEvent,
    ) -> Result<(), io::Error> {
        let task = &mut self.tasks[event.id];
        task.pb.println(format!(
            "Failed to compute checksum for {} ({})",
            event.resource, event.error
        ));

        Ok(())
    }
}

/// Logs events to a console.
#[derive(Default)]
pub struct Console {
    // Delay creation of the inner state until we receive our first BeginBuild
    // event. This lets us handle any number of threads.
    inner: Option<Inner>,
}

impl Console {
    fn style_idle() -> ProgressStyle {
        ProgressStyle::default_spinner().template("{prefix:.bold.dim} 🚶")
    }

    fn style_running() -> ProgressStyle {
        ProgressStyle::default_spinner().template(&format!(
            "{{prefix:.bold.dim}} 🏃 {} {{wide_msg}}",
            style("{elapsed}").dim()
        ))
    }

    pub fn new() -> Self {
        // Delay initialization until we receive a BeginBuild event.
        Self::default()
    }
}

impl EventHandler for Console {
    type Error = io::Error;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        match event {
            Event::BeginBuild(event) => {
                if self.inner.is_none() {
                    self.inner =
                        Some(Inner::new(event.threads, event.name, timestamp));
                }
            }
            Event::EndBuild(event) => {
                if let Some(inner) = self.inner.take() {
                    inner.end_build(timestamp, event)?;
                }
            }
            Event::BeginTask(event) => {
                if let Some(inner) = &mut self.inner {
                    inner.begin_task(timestamp, event)?;
                }
            }
            Event::TaskOutput(event) => {
                if let Some(inner) = &mut self.inner {
                    inner.tasks[event.id].buf.extend(event.chunk);
                }
            }
            Event::EndTask(event) => {
                if let Some(inner) = &mut self.inner {
                    inner.end_task(timestamp, event)?;
                }
            }
            Event::Delete(event) => {
                if let Some(inner) = &mut self.inner {
                    inner.delete(timestamp, event)?;
                }
            }
            Event::ChecksumError(event) => {
                if let Some(inner) = &mut self.inner {
                    inner.checksum_error(timestamp, event)?;
                }
            }
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        if let Some(inner) = self.inner.take() {
            inner.finish()?;
        }

        Ok(())
    }
}
