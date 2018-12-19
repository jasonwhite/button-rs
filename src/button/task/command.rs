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

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::{
    self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor,
};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::error::Error;
use crate::res;
use crate::util::{progress_dummy, Arguments, Process, Retry};

use super::traits::Task;
use crate::detect::{Detect, Detected};

const DEV_NULL: &str = "/dev/null";

/// A task that executes a single command. A command is simply a process to be
/// spawned.
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Command {
    /// Settings specific to spawning a process.
    process: Process,

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
    pub fn new(program: PathBuf, args: Arguments) -> Command {
        Command {
            process: Process::new(program, args),
            display: None,
            retry: None,
            detect: None,
        }
    }

    pub fn display(&mut self, display: String) -> &mut Command {
        self.display = Some(display);
        self
    }

    fn execute_impl(
        &self,
        root: &Path,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let detect = self
            .detect
            .unwrap_or_else(|| Detect::from_program(&self.process.program));

        let detected = detect.run(root, &self.process, log)?;

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

        // TODO: Analyze the command line and deduce inputs/outputs? For
        // example, we can easily see that, for the command `gcc foo.c -o
        // foo.o`, `foo.c` is an input and `foo.o` is an output. This could make
        // writing/generating the build description far more ergonomic.
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

// Implement Serialize manually because `#[serde(flatten)]` doesn't work with
// bincode. All of this can be removed if/when that is fixed.
impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("command", 5)?;
        state.serialize_field("program", &self.process.program)?;
        state.serialize_field("args", &self.process.args)?;
        state.serialize_field("cwd", &self.process.cwd)?;
        state.serialize_field("env", &self.process.env)?;
        state.serialize_field("stdin", &self.process.stdin)?;
        state.serialize_field("stdout", &self.process.stdout)?;
        state.serialize_field("stderr", &self.process.stderr)?;
        state.serialize_field("display", &self.display)?;
        state.serialize_field("retry", &self.retry)?;
        state.serialize_field("detect", &self.detect)?;
        state.end()
    }
}

// Implement Deserialize manually because `#[serde(flatten)]` doesn't work with
// bincode. All of this can be removed if/when that is fixed.
impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Command, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            Program,
            Args,
            Cwd,
            Env,
            Stdin,
            Stdout,
            Stderr,
            Display,
            Retry,
            Detect,
        }

        const FIELDS: &[&str] = &[
            "program", "args", "cwd", "env", "stdin", "stdout", "stderr",
            "display", "retry", "detect",
        ];

        struct CommandVisitor;

        impl<'de> Visitor<'de> for CommandVisitor {
            type Value = Command;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Command")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let program = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let args = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                let cwd = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;

                let env = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;

                let stdin = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;

                let stdout = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;

                let stderr = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;

                let display = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(8, &self))?;
                let retry = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(9, &self))?;
                let detect = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(10, &self))?;

                Ok(Command {
                    process: Process {
                        program,
                        args,
                        cwd,
                        env,
                        stdin,
                        stdout,
                        stderr,
                    },
                    display,
                    retry,
                    detect,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut program = None;
                let mut args = None;
                let mut cwd = None;
                let mut env = None;
                let mut stdin = None;
                let mut stdout = None;
                let mut stderr = None;
                let mut display = None;
                let mut retry = None;
                let mut detect = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Program => {
                            if program.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "program",
                                ));
                            }

                            program = Some(map.next_value()?);
                        }
                        Field::Args => {
                            if args.is_some() {
                                return Err(de::Error::duplicate_field("args"));
                            }

                            args = Some(map.next_value()?);
                        }
                        Field::Cwd => {
                            if cwd.is_some() {
                                return Err(de::Error::duplicate_field("cwd"));
                            }

                            cwd = Some(map.next_value()?);
                        }
                        Field::Env => {
                            if env.is_some() {
                                return Err(de::Error::duplicate_field("env"));
                            }

                            env = Some(map.next_value()?);
                        }
                        Field::Stdin => {
                            if stdin.is_some() {
                                return Err(de::Error::duplicate_field("stdin"));
                            }

                            stdin = Some(map.next_value()?);
                        }
                        Field::Stdout => {
                            if stdout.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "stdout",
                                ));
                            }

                            stdout = Some(map.next_value()?);
                        }
                        Field::Stderr => {
                            if stderr.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "stderr",
                                ));
                            }

                            stderr = Some(map.next_value()?);
                        }
                        Field::Display => {
                            if display.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "display",
                                ));
                            }

                            display = Some(map.next_value()?);
                        }
                        Field::Retry => {
                            if retry.is_some() {
                                return Err(de::Error::duplicate_field("retry"));
                            }

                            retry = Some(map.next_value()?);
                        }
                        Field::Detect => {
                            if detect.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "detect",
                                ));
                            }

                            detect = Some(map.next_value()?);
                        }
                    }
                }

                let program = program
                    .ok_or_else(|| de::Error::missing_field("program"))?;
                let args =
                    args.ok_or_else(|| de::Error::missing_field("args"))?;

                Ok(Command {
                    process: Process {
                        program,
                        args,
                        cwd,
                        env,
                        stdin,
                        stdout,
                        stderr,
                    },
                    display,
                    retry,
                    detect,
                })
            }
        }

        deserializer.deserialize_struct("command", FIELDS, CommandVisitor)
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
                    vec!["bar".into(), "baz".into()].into_iter().collect()
                )
            ),
            "foo bar baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec!["baz".into()].into_iter().collect()
                )
            ),
            "\"foo bar\" baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec!["some argument".into()].into_iter().collect()
                )
                .display(String::from("display this"))
            ),
            "display this"
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo"),
                    vec!["bar".into(), "baz".into()].into_iter().collect()
                )
            ),
            "foo bar baz"
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec!["baz".into()].into_iter().collect()
                )
            ),
            "\"foo bar\" baz"
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec!["some argument".into(), "with spaces".into()]
                        .into_iter()
                        .collect()
                )
                .display(String::from("display this"))
            ),
            "foo/bar/baz \"some argument\" \"with spaces\""
        );
    }
}
