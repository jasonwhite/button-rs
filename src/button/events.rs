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

use std::ops::Deref;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::detect::Detected;
use crate::res;
use crate::task;

/// A build has begun.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeginBuildEvent {
    /// The number of threads used during the build.
    pub threads: usize,
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
    pub chunk: Vec<u8>,
}

/// The task has finished.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndTaskEvent {
    /// The thread this task is getting executed on. This is stable
    /// throughout the execution of the task.
    pub id: usize,

    /// The result of this task.
    pub result: Result<Detected, Vec<String>>,
}

/// A resource is getting deleted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteEvent {
    /// The thread this deletion is getting executed on.
    pub thread: usize,

    /// The resource that is getting deleted.
    pub resource: res::Any,
}

/// A single build event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// A build has begun.
    BeginBuild(BeginBuildEvent),

    /// A task has started.
    BeginTask(BeginTaskEvent),

    /// A task has had output written to it.
    TaskOutput(TaskOutputEvent),

    /// The task has finished.
    EndTask(EndTaskEvent),

    /// A resource is getting deleted.
    Delete(DeleteEvent),
}

macro_rules! from_event {
    ($name:ident, $from:ident) => {
        impl From<$from> for Event {
            fn from(event: $from) -> Self {
                Event::$name(event)
            }
        }
    };
}

from_event!(BeginBuild, BeginBuildEvent);
from_event!(BeginTask, BeginTaskEvent);
from_event!(TaskOutput, TaskOutputEvent);
from_event!(EndTask, EndTaskEvent);
from_event!(Delete, DeleteEvent);

/// A wrapper to timestamp types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timestamped<T> {
    timestamp: DateTime<Utc>,
    inner: T,
}

impl<T> Timestamped<T> {
    /// Gets the timestamp of the event.
    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    /// Gets the wrapped event.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<T> for Timestamped<T> {
    fn from(inner: T) -> Self {
        Timestamped {
            timestamp: Utc::now(),
            inner,
        }
    }
}

impl<T> Deref for Timestamped<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub trait EventHandler {
    type Error: std::error::Error;

    /// Handles an event.
    fn send(&mut self, item: Event) -> Result<Event, Self::Error>;
}
