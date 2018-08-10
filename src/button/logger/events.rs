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

use res;
use task;

use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeginBuild {
    pub datetime: DateTime<Utc>,
    pub threads: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndBuild {
    pub datetime: DateTime<Utc>,
    pub result: Result<(), Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartTask {
    pub datetime: DateTime<Utc>,
    pub thread: usize,
    pub task: task::Any,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WriteTask {
    pub datetime: DateTime<Utc>,
    pub thread: usize,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FinishTask {
    pub datetime: DateTime<Utc>,
    pub thread: usize,
    pub result: Result<(), Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delete {
    pub datetime: DateTime<Utc>,
    pub thread: usize,
    pub resource: res::Any,
}

/// A logging event.
#[derive(Serialize, Deserialize, Debug, Clone)]
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
