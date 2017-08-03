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
use std::time;
use std::path::PathBuf;

use node::{Error, Task};

/// A task that executes a single command. A command is simply a process to be
/// spawned.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Command {
    /// Process and arguments to spawn.
    args: Vec<String>,

    /// Optional working directory to spawn the process in. If `None`, uses the
    /// working directory of the parent process (i.e., the build process).
    cwd: Option<PathBuf>,

    /// String to display when executing the task. If `None`, the command
    /// arguments are displayed in full instead.
    display: Option<String>,

    /// How much time to give the command to execute. If `None`, there is no
    /// time limit.
    timeout: Option<time::Duration>,

    /// How many times to retry the command before giving up. This is useful for
    /// flaky tests that may need to be run several times before succeeding.
    /// Between each execution, we wait a period of time. By default, an
    /// exponential backoff function is used to determine the wait duration.
    #[serde(default)]
    retries: u32,
}

impl Command {
    #[cfg(test)]
    pub fn new(args: Vec<String>) -> Command {
        Command {
            args: args,
            cwd: None,
            display: None,
            timeout: None,
            retries: 0,
        }
    }
}

impl Command {
    // Sets the working directory for the command.
    #[allow(dead_code)]
    pub fn cwd(mut self, path: PathBuf) -> Command {
        self.cwd = Some(path);
        self
    }

    // Sets the display string for the command.
    #[allow(dead_code)]
    pub fn display(mut self, display: String) -> Command {
        self.display = Some(display);
        self
    }

    // Sets the timeout for the command.
    #[allow(dead_code)]
    pub fn timeout(mut self, timeout: time::Duration) -> Command {
        self.timeout = Some(timeout);
        self
    }

    // Sets the number of retries for the command.
    #[allow(dead_code)]
    pub fn retries(mut self, retries: u32) -> Command {
        self.retries = retries;
        self
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref display) = self.display {
            write!(f, "{}", display)
        } else {
            // TODO: Format as a bash command.
            write!(f, "{:?}", self.args)
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.args)
    }
}

impl Task for Command {
    fn run(&self) -> Result<(), Error> {
        println!("Executing `{:?}` in directory {:?}", self.args, self.cwd);
        Ok(())
    }
}
