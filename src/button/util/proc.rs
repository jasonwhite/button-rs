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
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use os_pipe::{pipe, PipeReader};
use serde::{Deserialize, Serialize};
use tempfile::TempPath;

use crate::error::{Error, ResultExt};

use super::args::{Arg, Arguments};

#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug,
)]
pub struct Process {
    /// Program name.
    pub program: PathBuf,

    /// Program arguments.
    pub args: Arguments,

    /// Optional working directory to spawn the process in. If `None`, uses the
    /// working directory of the parent process (i.e., the build process).
    pub cwd: Option<PathBuf>,

    /// Optional environment variables.
    pub env: Option<BTreeMap<String, String>>,

    /// File to send to standard input. If `None`, the standard input stream
    /// reads from `/dev/null` or equivalent.
    pub stdin: Option<PathBuf>,

    /// Redirect standard output to a file instead. If the path is `/dev/null`,
    /// a cross-platform way of sending the output to a black hole is used. If
    /// `None`, the output is logged by this task.
    pub stdout: Option<PathBuf>,

    /// Redirect standard error to a file instead. If the path is `/dev/null`,
    /// a cross-platform way of sending the output to a black hole is
    /// used. If `None`, the output is logged by this task.
    pub stderr: Option<PathBuf>,
}

impl Process {
    /// If this path is given for stdin, stdout, or stderr, then I/O is
    /// redirected to a cross-platform blackhole.
    pub const DEV_NULL: &'static str = "/dev/null";

    pub fn new(program: PathBuf, args: Arguments) -> Process {
        Process {
            program,
            args,
            cwd: None,
            env: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    /// Replaces the arguments with a temporary response file and returns the
    /// temporary path.
    pub fn response_file(&mut self) -> Result<TempPath, io::Error> {
        let temp = self.args.response_file()?;

        let mut arg = OsString::new();
        arg.push("@");
        arg.push(&temp);

        // TODO: Handle the unlikely error.
        let mut args = Arguments::new();
        args.push(arg.into_string().unwrap().into());

        self.args = args;

        Ok(temp)
    }

    /// Creates the child process data structure, but does not spawn it.
    fn child(
        &self,
        root: &Path,
    ) -> Result<(PipeReader, process::Command), io::Error> {
        let mut child = process::Command::new(&self.program);

        if let Some(ref path) = self.stdin {
            if path == Path::new(Self::DEV_NULL) {
                child.stdin(process::Stdio::null());
            } else {
                child.stdin(fs::File::open(path)?);
            }
        } else {
            // We don't ever want the build system to pause waiting for user
            // input from the parent process' input stream.
            child.stdin(process::Stdio::null());
        }

        let (reader, writer) = pipe()?;

        {
            // Make sure the writer is dropped even if it isn't used below.
            // Otherwise, the parent process will hang reading from the child
            // process.
            let writer = writer;

            if let Some(ref path) = self.stdout {
                if path == Path::new(Self::DEV_NULL) {
                    // Use cross-platform method.
                    child.stdout(process::Stdio::null());
                } else {
                    child.stdout(fs::File::create(path)?);
                }
            } else {
                child.stdout(writer.try_clone()?);
            }

            if let Some(ref path) = self.stderr {
                if path == Path::new(Self::DEV_NULL) {
                    // Use cross-platform method.
                    child.stderr(process::Stdio::null());
                } else {
                    child.stderr(fs::File::create(path)?);
                }
            } else {
                child.stderr(writer);
            }
        }

        if let Some(ref cwd) = self.cwd {
            child.current_dir(root.join(cwd));
        } else if !root.as_os_str().is_empty() {
            child.current_dir(root);
        }

        if let Some(ref env) = self.env {
            child.envs(env);
        }

        child.args(&self.args);

        Ok((reader, child))
    }

    /// Creates and spawns the child process.
    ///
    /// The I/O pipes are handled in special ways:
    ///
    ///  - `stdin` is always redirected from `/dev/null` (or platform-specific
    ///    equivalent) unless a file path is given.
    ///  - `stderr` and `stdout` are always interleaved unless one (or both) are
    ///    redirected to a file path.
    pub fn spawn(&self, root: &Path) -> Result<(PipeReader, Child), Error> {
        let (reader, mut child) = self.child(root)?;

        let handle = child.spawn().context("Failed to spawn process")?;

        Ok((reader, Child(handle)))
    }
}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Arg::new(&self.program.to_string_lossy().as_ref()))?;

        for arg in &self.args {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

pub struct Child(process::Child);

impl Child {
    /// Wait for the child to exit. An error is returned if the process exited
    /// with a code other than 0.
    pub fn wait(mut self) -> Result<(), io::Error> {
        let status = self.0.wait()?;
        match status.code() {
            Some(code) => {
                if code == 0 {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Process exited with error code {}", code),
                    ))
                }
            }
            None => {
                // Handle signals on Unix platforms.
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;

                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Process terminated by signal {}",
                            status.signal().unwrap()
                        ),
                    ))
                }

                #[cfg(windows)]
                Ok(())
            }
        }
    }
}
