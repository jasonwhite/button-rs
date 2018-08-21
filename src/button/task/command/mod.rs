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

mod detect;

use self::detect::{Detect, Process};

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use os_pipe;

use error::{Error, ResultExt};

use super::traits::Task;
use util::NeverAlwaysAuto;

use res;
use util::{progress_dummy, Arg, Arguments, Retry};

const DEV_NULL: &str = "/dev/null";

/// A task that executes a single command. A command is simply a process to be
/// spawned.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
pub struct Command {
    /// Program name.
    program: PathBuf,

    /// Program arguments.
    args: Arguments,

    /// Optional working directory to spawn the process in. If `None`, uses the
    /// working directory of the parent process (i.e., the build process).
    cwd: Option<PathBuf>,

    /// Optional environment variables.
    env: Option<BTreeMap<String, String>>,

    /// Response file creation.
    ///
    /// If `Never`, never creates a response file. If the command line length
    /// exceeds the operating system limits, the command will fail.
    ///
    /// If `Always`, creates a temporary response file with all the command
    /// line arguments and passes that as the first command line argument
    /// instead. This is useful for very long command lines that exceed
    /// operating system limits.
    ///
    /// If `Auto`, creates a temporary response file only if the size of the
    /// arguments exceeds the operating system limits.
    #[serde(default)]
    response_file: NeverAlwaysAuto,

    /// File to send to standard input. If `None`, the standard input stream
    /// reads from `/dev/null` or equivalent.
    stdin: Option<PathBuf>,

    /// Redirect standard output to a file instead. If the path is `/dev/null`,
    /// a cross-platform way of sending the output to a black hole is used. If
    /// `None`, the output is logged by this task.
    stdout: Option<PathBuf>,

    /// Redirect standard error to a file instead. If the path is `/dev/null`,
    /// a cross-platform way of sending the output to a black hole is
    /// used. If `None`, the output is logged by this task.
    stderr: Option<PathBuf>,

    /// String to display when executing the task. If `None`, the command
    /// arguments are displayed in full instead.
    display: Option<String>,

    /// Retry settings.
    retry: Option<Retry>,

    /// Input and output detection
    detect: Option<Detect>,
}

impl Command {
    #[cfg(test)]
    pub fn new(program: PathBuf, args: Arguments) -> Box<Command> {
        Box::new(Command {
            program: program,
            args: args,
            cwd: None,
            env: None,
            response_file: NeverAlwaysAuto::default(),
            stdin: None,
            stdout: None,
            stderr: None,
            display: None,
            retry: None,
        })
    }
}

impl Command {
    // Sets the working directory for the command.
    #[allow(dead_code)]
    pub fn cwd(&mut self, path: PathBuf) -> &mut Command {
        self.cwd = Some(path);
        self
    }

    // Sets the stdout file for the command.
    #[allow(dead_code)]
    pub fn stdout(&mut self, path: PathBuf) -> &mut Command {
        self.stdout = Some(path);
        self
    }

    // Sets the display string for the command.
    #[allow(dead_code)]
    pub fn display(&mut self, display: String) -> &mut Command {
        self.display = Some(display);
        self
    }

    // Sets the retry configuration.
    #[allow(dead_code)]
    pub fn retry(&mut self, retry: Retry) -> &mut Command {
        self.retry = Some(retry);
        self
    }

    fn create_process(&self, root: &Path) -> Result<Process, Error> {
        let mut child = process::Command::new(&self.program);

        if let Some(ref path) = self.stdin {
            if path == Path::new(DEV_NULL) {
                child.stdin(process::Stdio::null());
            } else {
                child.stdin(fs::File::open(path)?);
            }
        } else {
            // We don't ever want the build system to pause waiting for user
            // input from the parent process' input stream.
            child.stdin(process::Stdio::null());
        }

        let (reader, writer) = os_pipe::pipe()?;

        {
            // Make sure the writer is dropped even if it isn't used below.
            // Otherwise, the parent process will hang reading from the child
            // process.
            let writer = writer;

            if let Some(ref path) = self.stdout {
                if path == Path::new(DEV_NULL) {
                    // Use cross-platform method.
                    child.stdout(process::Stdio::null());
                } else {
                    child.stdout(fs::File::create(path)?);
                }
            } else {
                child.stdout(writer.try_clone()?);
            }

            if let Some(ref path) = self.stderr {
                if path == Path::new(DEV_NULL) {
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

        // Generate a response file if necessary.
        let generate_response_file = match self.response_file {
            NeverAlwaysAuto::Never => false,
            NeverAlwaysAuto::Always => true,
            NeverAlwaysAuto::Auto => self.args.is_too_large(),
        };

        let response_file = if generate_response_file {
            let temp = self
                .args
                .response_file()
                .context("Failed writing response file")?;

            let mut arg = OsString::new();
            arg.push("@");
            arg.push(&temp);
            child.arg(&arg);

            Some(temp)
        } else {
            child.args(&self.args);
            None
        };

        Ok(Process::new(child, reader, response_file))
    }

    fn execute_impl(
        &self,
        root: &Path,
        log: &mut io::Write,
    ) -> Result<(), Error> {
        let process = self.create_process(root)?;

        let detect = self
            .detect
            .unwrap_or_else(|| Detect::from_program(&self.program));

        let detected = detect.run(root, process, log)?;

        for p in detected.inputs() {
            writeln!(log, "Input: {:?}", p)?;
        }

        for p in detected.outputs() {
            writeln!(log, "Output: {:?}", p)?;
        }

        Ok(())
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref display) = self.display {
            write!(f, "{}", display)
        } else {
            write!(
                f,
                "{}",
                Arg::new(&self.program.to_string_lossy().as_ref())
            )?;

            for arg in &self.args {
                write!(f, " {}", arg)?;
            }

            Ok(())
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"")?;

        write!(f, "{}", Arg::new(&self.program.to_string_lossy().as_ref()))?;

        for arg in &self.args {
            write!(f, " {}", arg)?;
        }

        write!(f, "\"")?;

        Ok(())
    }
}

impl Task for Command {
    fn execute(&self, root: &Path, log: &mut io::Write) -> Result<(), Error> {
        if let Some(ref retry) = self.retry {
            retry.call(|| self.execute_impl(root, log), progress_dummy)
        } else {
            self.execute_impl(root, log)
        }
    }

    fn known_inputs(&self, set: &mut res::Set) {
        set.insert(self.program.clone().into());

        if let Some(ref path) = self.stdin {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }

        // Depend on the working directory.
        if let Some(ref path) = self.cwd {
            set.insert(res::Dir::new(path.clone()).into());
        }

        // Depend on parent directory of the stdout file.
        if let Some(ref path) = self.stdout {
            if path != Path::new(DEV_NULL) {
                if let Some(parent) = path.parent() {
                    set.insert(res::Dir::new(parent.to_path_buf()).into());
                }
            }
        }

        // Depend on parent directory of the stderr file.
        if let Some(ref path) = self.stderr {
            if path != Path::new(DEV_NULL) {
                if let Some(parent) = path.parent() {
                    set.insert(res::Dir::new(parent.to_path_buf()).into());
                }
            }
        }
    }

    fn known_outputs(&self, set: &mut res::Set) {
        if let Some(ref path) = self.stdout {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }

        if let Some(ref path) = self.stderr {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_display() {
        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo"),
                    vec!["bar", "baz"].iter().collect()
                )
            ),
            "foo bar baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec!["baz"].iter().collect()
                )
            ),
            "\"foo bar\" baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec!["some argument"].iter().collect()
                ).display(String::from("display this"))
            ),
            "display this"
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo"),
                    vec!["bar", "baz"].iter().collect()
                )
            ),
            "\"foo bar baz\""
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec!["baz"].iter().collect()
                )
            ),
            "\"\"foo bar\" baz\""
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec!["some argument"].iter().collect()
                ).display(String::from("display this"))
            ),
            "\"foo/bar/baz \"some argument\"\""
        );
    }
}
