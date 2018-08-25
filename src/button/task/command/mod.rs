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

use self::detect::Detect;

use std::fmt;
use std::io;
use std::path::Path;

use error::Error;

use super::traits::{Detected, Task};
use util::{NeverAlwaysAuto, Process};

use res;
use util::{progress_dummy, Retry};

const DEV_NULL: &str = "/dev/null";

/// A task that executes a single command. A command is simply a process to be
/// spawned.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
pub struct Command {
    /// Settings specific to spawning a process.
    #[serde(flatten)]
    process: Process,

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
    pub response_file: NeverAlwaysAuto,

    /// String to display when executing the task. If `None`, the command
    /// arguments are displayed in full instead.
    display: Option<String>,

    /// Retry settings.
    retry: Option<Retry>,

    /// Input and output detection. If not specified, determines the detection
    /// method based on the program name.
    detect: Option<Detect>,
}

impl Command {
    fn execute_impl(
        &self,
        root: &Path,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let detect = self
            .detect
            .unwrap_or_else(|| Detect::from_program(&self.process.program));

        let detected = detect.run(root, &self.process, log)?;

        for p in detected.inputs() {
            writeln!(log, "Input: {:?}", p)?;
        }

        for p in detected.outputs() {
            writeln!(log, "Output: {:?}", p)?;
        }

        Ok(detected)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref display) = self.display {
            write!(f, "{}", display)
        } else {
            write!(f, "{}", self.process)
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.process)
    }
}

impl Task for Command {
    fn execute(
        &self,
        root: &Path,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        if let Some(ref retry) = self.retry {
            retry.call(|| self.execute_impl(root, log), progress_dummy)
        } else {
            self.execute_impl(root, log)
        }
    }

    fn known_inputs(&self, set: &mut res::Set) {
        set.insert(self.process.program.clone().into());

        if let Some(ref path) = self.process.stdin {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }

        // Depend on the working directory.
        if let Some(ref path) = self.process.cwd {
            set.insert(res::Dir::new(path.clone()).into());
        }

        // Depend on parent directory of the stdout file.
        if let Some(ref path) = self.process.stdout {
            if path != Path::new(DEV_NULL) {
                if let Some(parent) = path.parent() {
                    set.insert(res::Dir::new(parent.to_path_buf()).into());
                }
            }
        }

        // Depend on parent directory of the stderr file.
        if let Some(ref path) = self.process.stderr {
            if path != Path::new(DEV_NULL) {
                if let Some(parent) = path.parent() {
                    set.insert(res::Dir::new(parent.to_path_buf()).into());
                }
            }
        }
    }

    fn known_outputs(&self, set: &mut res::Set) {
        if let Some(ref path) = self.process.stdout {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }

        if let Some(ref path) = self.process.stderr {
            if path != Path::new(DEV_NULL) {
                set.insert(path.clone().into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
