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

use std::fmt;
use std::io;

use super::command::Command;
use super::copy::Copy;
use super::download::Download;
use super::makedir::MakeDir;
use super::traits::{Error, Task};

use res;

/// Any possible task. This list is used for deserialization purposes.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Any {
    /// A single command execution.
    Command(Box<Command>),

    /// Download something.
    Download(Download),

    /// Create a directory.
    MakeDir(MakeDir),

    /// Copy a file.
    Copy(Copy),
}

impl fmt::Display for Any {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Any::Command(ref x) => x.fmt(f),
            &Any::Download(ref x) => x.fmt(f),
            &Any::MakeDir(ref x) => x.fmt(f),
            &Any::Copy(ref x) => x.fmt(f),
        }
    }
}

impl fmt::Debug for Any {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Any::Command(ref x) => x.fmt(f),
            &Any::Download(ref x) => x.fmt(f),
            &Any::MakeDir(ref x) => x.fmt(f),
            &Any::Copy(ref x) => x.fmt(f),
        }
    }
}

impl From<Command> for Any {
    fn from(res: Command) -> Self {
        Any::Command(Box::new(res))
    }
}

impl From<Box<Command>> for Any {
    fn from(res: Box<Command>) -> Self {
        Any::Command(res)
    }
}

impl From<Download> for Any {
    fn from(res: Download) -> Self {
        Any::Download(res)
    }
}

impl From<MakeDir> for Any {
    fn from(res: MakeDir) -> Self {
        Any::MakeDir(res)
    }
}

impl From<Copy> for Any {
    fn from(res: Copy) -> Self {
        Any::Copy(res)
    }
}

impl Task for Any {
    fn execute(&self, log: &mut io::Write) -> Result<(), Error> {
        match self {
            &Any::Command(ref x) => x.execute(log),
            &Any::Download(ref x) => x.execute(log),
            &Any::MakeDir(ref x) => x.execute(log),
            &Any::Copy(ref x) => x.execute(log),
        }
    }

    fn known_inputs(&self, set: &mut res::Set) {
        match self {
            &Any::Command(ref x) => x.known_inputs(set),
            &Any::Download(ref x) => x.known_inputs(set),
            &Any::MakeDir(ref x) => x.known_inputs(set),
            &Any::Copy(ref x) => x.known_inputs(set),
        }
    }

    fn known_outputs(&self, set: &mut res::Set) {
        match self {
            &Any::Command(ref x) => x.known_outputs(set),
            &Any::Download(ref x) => x.known_outputs(set),
            &Any::MakeDir(ref x) => x.known_outputs(set),
            &Any::Copy(ref x) => x.known_outputs(set),
        }
    }
}
