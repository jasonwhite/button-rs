// Copyright (c) 2017 Jason White
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

use std::time::Duration;

pub struct BeginEvent {
    /// Number of threads used for this build.
    threads: usize,
}

pub struct EndEvent {
    /// The result of the build.
    result: Result<(), String>,

    /// How long the build took to finish.
    duration: Duration,
}

pub struct TaskStart {
    /// The thread that this task is being executed on. This ID will always be
    /// less than the total number of threads being used. Thus, a vector can be
    /// used to hold information about the currently executing tasks.
    thread: usize,
}

pub struct TaskOutput<'a> {
    /// The thread that this task is being executed on. This ID will always be
    /// less than the total number of threads being used. Thus, a vector can be
    /// used to hold information about the currently executing tasks.
    thread: usize,

    /// The chunk of output.
    chunk: &'a [u8],
}

pub struct TaskEnd {
    /// The thread that this task is being executed on. This ID will always be
    /// less than the total number of threads being used. Thus, a vector can be
    /// used to hold information about the currently executing tasks.
    thread: usize,

    /// The result of the task execution.
    result: Result<(), String>,

    /// The amount of time it took to execute this task.
    duration: Duration,
}

/// Represents an event that has occurred in the build.
///
/// A stream of these events is used to log the output of the build. Such a
/// stream could also be serialized to disk for later playback or analysis.
enum Event<'a> {
    /// The build has started.
    Begin(BeginEvent),

    /// The build has finished. This will always be the last event to be sent.
    End(EndEvent),

    /// A task has started executing.
    TaskStart(TaskStart),

    /// A chunk of output from a task. There may be 0 or more of these events
    /// between a task starting and a task ending.
    TaskOutput(TaskOutput<'a>),

    /// A task has finished executing.
    TaskEnd(TaskEnd),
}
