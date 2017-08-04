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

mod command;
mod download;

pub use self::command::Command;
pub use self::download::Download;

use std::fmt;
use std::io;
use node;

/// Complete list of task types. This list is used for derserialization
/// purposes.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Task {
    /// A single command execution.
    Command(Command),

    /// Download something.
    Download(Download),
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Task::Command(ref x) => x.fmt(f),
            &Task::Download(ref x) => x.fmt(f),
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Task::Command(ref x) => x.fmt(f),
            &Task::Download(ref x) => x.fmt(f),
        }
    }
}

impl node::Task for Task {
    fn retries(&self) -> u32 {
        match self {
            &Task::Command(ref x) => x.retries(),
            &Task::Download(ref x) => x.retries(),
        }
    }

    fn run(&self, log: &mut io::Write) -> Result<(), node::Error> {
        match self {
            &Task::Command(ref x) => x.run(log),
            &Task::Download(ref x) => x.run(log),
        }
    }
}