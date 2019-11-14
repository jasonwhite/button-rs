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

//! The build itself does not handle any sort of display for the user. Instead,
//! it sends events along a channel for consumption by a user-facing logging
//! system. This way, events can be sent across the network transparently.

use std::fmt;
use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use bincode;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

use crate::detect::Detected;
use crate::res;
use crate::task;

mod binary;
mod console;

pub use self::binary::Binary;
pub use self::console::Console;

/// A build has begun.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeginBuildEvent {
    /// The number of threads used during the build.
    pub threads: usize,

    /// The name of the build.
    pub name: String,
}

/// A build has ended.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndBuildEvent {
    /// The result of the build.
    pub result: Result<(), String>,
}

/// A task has started executing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeginTaskEvent {
    /// The thread this task is getting executed on. This is stable
    /// throughout the execution of the task.
    pub id: usize,

    /// The task that has started. If this needs to be stored for later, use
    /// the thread number in conjunction with the total number of threads to
    /// store task information in a `Vec`.
    pub task: task::Any,
}

/// A task has had output written to it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskOutputEvent {
    /// The thread this task is getting executed on. This is stable
    /// throughout the execution of the task.
    pub id: usize,

    /// The chunk of data that has been output by the task.
    pub chunk: Bytes,
}

/// The task has finished.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndTaskEvent {
    /// The thread this task is getting executed on. This is stable
    /// throughout the execution of the task.
    pub id: usize,

    /// The result of this task.
    pub result: Result<Detected, String>,
}

/// A resource is getting deleted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteEvent {
    /// The thread this deletion is getting executed on.
    pub id: usize,

    /// The resource that is getting deleted.
    pub resource: res::Any,

    /// The stringy result of the deletion.
    pub result: Result<(), String>,
}

/// The checksum of a resource failed to compute.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChecksumErrorEvent {
    /// The thread this deletion is getting executed on.
    pub id: usize,

    /// The resource that is getting deleted.
    pub resource: res::Any,

    /// The stringy error of the deletion.
    pub error: String,
}

/// A single build event.
#[derive(Clone, Debug, Serialize, Deserialize, From)]
pub enum Event {
    /// A build has begun.
    BeginBuild(BeginBuildEvent),

    /// A build has finished.
    EndBuild(EndBuildEvent),

    /// A task has started.
    BeginTask(BeginTaskEvent),

    /// A task has had output written to it.
    TaskOutput(TaskOutputEvent),

    /// The task has finished.
    EndTask(EndTaskEvent),

    /// A resource is getting deleted.
    Delete(DeleteEvent),

    /// The checksum of a resource failed to compute.
    ChecksumError(ChecksumErrorEvent),
}

pub type Timestamp = DateTime<Utc>;

pub type EventSender = Sender<(Timestamp, Event)>;
pub type EventReceiver = Receiver<(Timestamp, Event)>;

/// Trait for receiving timestamped events.
///
/// Implementors of this can do interesting things like:
///
///  - Write the events to stdout.
///  - Write to a web page.
///  - Write to a binary log file for later replay.
///  - Send the events to another process for consumption.
///  - Forward to another event handler.
pub trait EventHandler: Send {
    type Error: std::error::Error;

    /// Listens for events on a channel, sending them to an event handler. If
    /// the event handler returns an error, this function stops listening for
    /// events and returns the error as well.
    ///
    /// This function returns when the sending channel has hung up (i.e., all
    /// senders have been dropped).
    fn read_channel(
        &mut self,
        receiver: EventReceiver,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        for (timestamp, event) in receiver.iter() {
            self.call(timestamp, event)?;
        }

        self.finish()
    }

    /// Gets events from a reader. Stops reading when an error occurs.
    ///
    /// If `realtime` is `true`, then an appropriate amount of time is waited
    /// between each event.
    fn read_bincode<R>(
        &mut self,
        mut reader: R,
        realtime: bool,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        R: io::Read,
    {
        // Grab the first event. We need the initial timestamp to calculate
        // sleep deltas when doing realtime playback.
        let (mut prev, event) = match bincode::deserialize_from(&mut reader) {
            Ok(x) => x,
            Err(_) => return self.finish(),
        };

        self.call(prev, event)?;

        while let Ok((timestamp, event)) =
            bincode::deserialize_from::<_, (Timestamp, _)>(&mut reader)
        {
            if realtime {
                if let Ok(delta) =
                    timestamp.signed_duration_since(prev).to_std()
                {
                    thread::sleep(delta);
                }
            }

            prev = timestamp;
            self.call(timestamp, event)?;
        }

        self.finish()
    }

    /// Handles an event.
    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error>;

    /// Called when there are no more events.
    fn finish(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<H> EventHandler for Box<H>
where
    H: EventHandler + ?Sized,
{
    type Error = H::Error;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        (**self).call(timestamp, event)
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        (**self).finish()
    }
}

impl<H> EventHandler for [H]
where
    H: EventHandler,
{
    type Error = H::Error;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        for handler in self.iter_mut() {
            handler.call(timestamp, event.clone())?;
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        for handler in self.iter_mut() {
            handler.finish()?;
        }

        Ok(())
    }
}

impl<H> EventHandler for Vec<H>
where
    H: EventHandler,
{
    type Error = H::Error;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        for handler in self.iter_mut() {
            handler.call(timestamp, event.clone())?;
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        for handler in self.iter_mut() {
            handler.finish()?;
        }

        Ok(())
    }
}

#[derive(Debug, Display)]
pub enum AnyHandlerError {
    Binary(<Binary as EventHandler>::Error),
    Console(<Console as EventHandler>::Error),
}

impl std::error::Error for AnyHandlerError {}

#[derive(From)]
pub enum AnyHandler {
    Binary(Binary),
    Console(Console),
}

impl EventHandler for AnyHandler {
    type Error = AnyHandlerError;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Binary(h) => {
                h.call(timestamp, event).map_err(AnyHandlerError::Binary)
            }
            Self::Console(h) => {
                h.call(timestamp, event).map_err(AnyHandlerError::Console)
            }
        }
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        match self {
            Self::Binary(h) => h.finish().map_err(AnyHandlerError::Binary),
            Self::Console(h) => h.finish().map_err(AnyHandlerError::Console),
        }
    }
}

/// A helper trait for sending events to a sink.
pub trait EventSink {
    /// Sends a `BeginBuildEvent` to the sink.
    fn begin_build<S>(&self, threads: usize, name: S)
    where
        S: Into<String>;

    /// Sends a `EndBuildEvent` to the sink.
    fn end_build<E>(&self, result: &Result<(), E>)
    where
        E: fmt::Display;

    /// Sends a `BeginTaskEvent` to the sink and returns an output writer.
    fn begin_task(&self, id: usize, task: task::Any)
        -> TaskOutputWriter<&Self>;

    /// Sends a `TaskOutputEvent` to the sink.
    fn task_output(&self, id: usize, chunk: Bytes);

    /// Sends a `EndTaskEvent` to the sink.
    fn end_task<E>(&self, id: usize, result: &Result<Detected, E>)
    where
        E: fmt::Display;

    /// Sends a `DeleteEvent` to the sink.
    fn delete<E>(&self, id: usize, resource: res::Any, result: &Result<(), E>)
    where
        E: fmt::Display;

    /// Sends a `ChecksumErrorEvent` to the sink.
    fn checksum_error<E>(&self, id: usize, resource: res::Any, error: &E)
    where
        E: fmt::Display;
}

// TODO: Don't unwrap. Log the errors instead.
impl EventSink for EventSender {
    fn begin_build<S>(&self, threads: usize, name: S)
    where
        S: Into<String>,
    {
        let event = BeginBuildEvent {
            threads,
            name: name.into(),
        };
        self.send((Utc::now(), Event::BeginBuild(event))).unwrap();
    }

    fn end_build<E>(&self, result: &Result<(), E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => Err(err.to_string()),
        };

        let event = EndBuildEvent { result };
        self.send((Utc::now(), Event::EndBuild(event))).unwrap();
    }

    fn begin_task(
        &self,
        id: usize,
        task: task::Any,
    ) -> TaskOutputWriter<&Self> {
        let event = BeginTaskEvent { id, task };
        self.send((Utc::now(), Event::BeginTask(event))).unwrap();

        TaskOutputWriter { id, sink: self }
    }

    fn task_output(&self, id: usize, chunk: Bytes) {
        let event = TaskOutputEvent { id, chunk };
        self.send((Utc::now(), Event::TaskOutput(event))).unwrap();
    }

    fn end_task<E>(&self, id: usize, result: &Result<Detected, E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(x) => Ok(x.clone()),
            Err(err) => Err(err.to_string()),
        };

        let event = EndTaskEvent {
            id,
            result: result.map_err(|e| e.to_string()),
        };
        self.send((Utc::now(), Event::EndTask(event))).unwrap();
    }

    fn delete<E>(&self, id: usize, resource: res::Any, result: &Result<(), E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => Err(err.to_string()),
        };

        let event = DeleteEvent {
            id,
            resource,
            result,
        };
        self.send((Utc::now(), Event::Delete(event))).unwrap();
    }

    fn checksum_error<E>(&self, id: usize, resource: res::Any, error: &E)
    where
        E: fmt::Display,
    {
        let event = ChecksumErrorEvent {
            id,
            resource,
            error: error.to_string(),
        };

        self.send((Utc::now(), Event::ChecksumError(event)))
            .unwrap();
    }
}

impl<'a> EventSink for &'a EventSender {
    fn begin_build<S>(&self, threads: usize, name: S)
    where
        S: Into<String>,
    {
        let event = BeginBuildEvent {
            threads,
            name: name.into(),
        };
        self.send((Utc::now(), Event::BeginBuild(event))).unwrap();
    }

    fn end_build<E>(&self, result: &Result<(), E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => Err(err.to_string()),
        };

        let event = EndBuildEvent { result };
        self.send((Utc::now(), Event::EndBuild(event))).unwrap();
    }

    fn begin_task(
        &self,
        id: usize,
        task: task::Any,
    ) -> TaskOutputWriter<&Self> {
        let event = BeginTaskEvent { id, task };
        self.send((Utc::now(), Event::BeginTask(event))).unwrap();

        TaskOutputWriter { id, sink: self }
    }

    fn task_output(&self, id: usize, chunk: Bytes) {
        let event = TaskOutputEvent { id, chunk };
        self.send((Utc::now(), Event::TaskOutput(event))).unwrap();
    }

    fn end_task<E>(&self, id: usize, result: &Result<Detected, E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(x) => Ok(x.clone()),
            Err(err) => Err(err.to_string()),
        };

        let event = EndTaskEvent {
            id,
            result: result.map_err(|e| e.to_string()),
        };
        self.send((Utc::now(), Event::EndTask(event))).unwrap();
    }

    fn delete<E>(&self, id: usize, resource: res::Any, result: &Result<(), E>)
    where
        E: fmt::Display,
    {
        let result = match result {
            Ok(()) => Ok(()),
            Err(err) => Err(err.to_string()),
        };

        let event = DeleteEvent {
            id,
            resource,
            result,
        };
        self.send((Utc::now(), Event::Delete(event))).unwrap();
    }

    fn checksum_error<E>(&self, id: usize, resource: res::Any, error: &E)
    where
        E: fmt::Display,
    {
        let event = ChecksumErrorEvent {
            id,
            resource,
            error: error.to_string(),
        };

        self.send((Utc::now(), Event::ChecksumError(event)))
            .unwrap();
    }
}

/// Helper for writing task output more ergonomically.
pub struct TaskOutputWriter<S> {
    id: usize,
    sink: S,
}

impl<S> TaskOutputWriter<S>
where
    S: EventSink,
{
    pub fn finish<E>(self, result: &Result<Detected, E>)
    where
        E: fmt::Display,
    {
        self.sink.end_task(self.id, result);
    }
}

impl<S> io::Write for TaskOutputWriter<S>
where
    S: EventSink,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO: Do buffering?
        self.sink.task_output(self.id, Bytes::from(buf));
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.sink.task_output(self.id, Bytes::from(buf));
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        // noop
        Ok(())
    }
}

/// Helper for creating and destroying an event handler thread that receives
/// events.
pub struct EventThread<H>
where
    H: EventHandler,
{
    handle: Option<JoinHandle<Result<H, H::Error>>>,
}

impl<H> EventThread<H>
where
    H: EventHandler,
{
    pub fn new(mut handler: H, receiver: EventReceiver) -> Self
    where
        H: EventHandler + Send + 'static,
        H::Error: Send,
    {
        EventThread {
            handle: Some(thread::spawn(move || {
                handler.read_channel(receiver)?;
                Ok(handler)
            })),
        }
    }

    pub fn join(mut self) -> Result<H, H::Error> {
        let handle = self.handle.take().unwrap();
        handle.join().unwrap()
    }
}

impl<H> Drop for EventThread<H>
where
    H: EventHandler,
{
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join().unwrap();
        }
    }
}

/// An event handler that simply keeps a count for each event. Useful for
/// testing.
#[cfg(test)]
mod test {
    use super::*;
    use std::sync::mpsc;

    #[derive(Default)]
    pub struct Stat {
        pub begin_build: usize,
        pub end_build: usize,
        pub begin_task: usize,
        pub task_output: usize,
        pub end_task: usize,
        pub delete: usize,
        pub checksum_error: usize,
    }

    impl EventHandler for Stat {
        type Error = io::Error;

        fn call(
            &mut self,
            _timestamp: Timestamp,
            event: Event,
        ) -> Result<(), Self::Error> {
            match event {
                Event::BeginBuild(_) => {
                    self.begin_build += 1;
                }
                Event::EndBuild(_) => {
                    self.end_build += 1;
                }
                Event::BeginTask(_) => {
                    self.begin_task += 1;
                }
                Event::TaskOutput(_) => {
                    self.task_output += 1;
                }
                Event::EndTask(_) => {
                    self.end_task += 1;
                }
                Event::Delete(_) => {
                    self.delete += 1;
                }
                Event::ChecksumError(_) => {
                    self.checksum_error += 1;
                }
            }

            Ok(())
        }
    }

    #[test]
    fn event_handler() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, receiver) = mpsc::channel();

        let event_thread = EventThread::new(Stat::default(), receiver);

        sender.begin_build(42, "build");

        let result: Result<(), &str> = Ok(());
        sender.end_build(&result);

        drop(sender);

        let stats = event_thread.join()?;

        assert_eq!(stats.begin_build, 1);
        assert_eq!(stats.end_build, 1);
        assert_eq!(stats.begin_task, 0);
        assert_eq!(stats.task_output, 0);
        assert_eq!(stats.end_task, 0);
        assert_eq!(stats.delete, 0);
        assert_eq!(stats.checksum_error, 0);

        Ok(())
    }
}
