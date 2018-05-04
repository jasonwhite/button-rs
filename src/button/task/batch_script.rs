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

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process;

use tempfile;

use super::traits::{Error, Task};

use retry;

/// A task to create a directory.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct BatchScript {
    /// Contents of the batch script. This is first written to a temporary file
    /// and then executed.
    contents: String,

    /// Optional working directory to spawn the process in. If `None`, uses the
    /// working directory of the parent process (i.e., the build process).
    cwd: Option<PathBuf>,

    /// Optional environment variables.
    env: Option<BTreeMap<String, String>>,

    /// String to display when executing the task.
    display: Option<String>,

    /// Turn echo off
    #[serde(default)]
    quiet: bool,

    /// Retry settings.
    retry: Option<retry::Retry>,
}

impl BatchScript {
    fn execute_impl(&self, log: &mut io::Write) -> Result<(), Error> {

        let mut cmd = process::Command::new("cmd.exe");

        // Don't allow user input.
        cmd.stdin(process::Stdio::null());

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(ref env) = self.env {
            cmd.envs(env);
        }

        // Write the script contents to a temporary file for execution.
        let temppath = {
            let mut tmp = tempfile::Builder::new()
                .suffix(".bat")
                .tempfile()?;
            tmp.as_file_mut().write(self.contents.as_bytes())?;
            tmp.into_temp_path()
        };

        if self.quiet {
            cmd.arg("/Q");
        }

        cmd.args(&["/c", "call", temppath.to_str().unwrap()]);

        let output = cmd.output()?;

        // TODO: Interleave stdout and stderr.
        log.write(&output.stdout)?;
        log.write(&output.stderr)?;

        if output.status.success() {
            Ok(())
        } else {
            match output.status.code() {
                Some(code) => {
                    Err(io::Error::new(io::ErrorKind::Other,
                                       format!("Process exited with error code {}",
                                               code)))
                }
                None => {
                    Err(io::Error::new(io::ErrorKind::Other,
                                       "Process terminated by signal"))
                }
            }
        }
    }
}

impl fmt::Display for BatchScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.display {
            Some(ref display) => write!(f, "batch script: {}", display),
            None => write!(f, "batch script"),
        }
    }
}

impl fmt::Debug for BatchScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for BatchScript {
    fn execute(&self, log: &mut io::Write) -> Result<(), Error> {
        if let Some(ref retry) = self.retry {
            retry.call(|| self.execute_impl(log), retry::progress_dummy)
        } else {
            self.execute_impl(log)
        }
    }
}

