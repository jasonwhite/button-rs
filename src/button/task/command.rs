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
use std::ffi::OsString;
use std::fmt;
#[cfg(windows)]
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;

use error::{Error, ResultExt};
use tempfile::{NamedTempFile, TempPath};

use super::traits::Task;
use util::NeverAlwaysAuto;

use res;
use util::{progress_dummy, Retry};

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
    args: Vec<String>,

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

    /// How much time to give the command to execute. If `None`, there is no
    /// time limit.
    timeout: Option<Duration>,

    /// Retry settings.
    retry: Option<Retry>,
}

impl Command {
    #[cfg(test)]
    pub fn new(program: PathBuf, args: Vec<String>) -> Box<Command> {
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
            timeout: None,
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

    // Sets the timeout for the command.
    #[allow(dead_code)]
    pub fn timeout(&mut self, timeout: Duration) -> &mut Command {
        self.timeout = Some(timeout);
        self
    }

    // Sets the retry configuration.
    #[allow(dead_code)]
    pub fn retry(&mut self, retry: Retry) -> &mut Command {
        self.retry = Some(retry);
        self
    }

    fn execute_impl(&self, root: &Path, log: &mut io::Write) -> Result<(), Error> {
        // TODO:
        //  1. Spawn the process
        //  2. Capture stdout/stderr appropriately.
        //  4. Add implicit dependency detection framework and refactor all of
        //     the above to make it work.
        //  5. Implement timeouts.

        let mut cmd = process::Command::new(&self.program);

        if let Some(ref path) = self.stdin {
            if path == Path::new(DEV_NULL) {
                cmd.stdin(process::Stdio::null());
            } else {
                cmd.stdin(fs::File::open(path)?);
            }
        } else {
            // We don't ever want the build system to pause waiting for user
            // input from the parent process' input stream.
            cmd.stdin(process::Stdio::null());
        }

        if let Some(ref path) = self.stdout {
            if path == Path::new(DEV_NULL) {
                // Use cross-platform method.
                cmd.stdout(process::Stdio::null());
            } else {
                cmd.stdout(fs::File::create(path)?);
            }
        }

        if let Some(ref path) = self.stderr {
            if path == Path::new(DEV_NULL) {
                // Use cross-platform method.
                cmd.stderr(process::Stdio::null());
            } else {
                cmd.stderr(fs::File::create(path)?);
            }
        }

        // Generate a response file if necessary.
        let generate_response_file = match self.response_file {
            NeverAlwaysAuto::Never => false,
            NeverAlwaysAuto::Always => true,
            NeverAlwaysAuto::Auto => args_too_large(&self.args),
        };

        // The temporary response file needs to outlive the spawned process, so
        // it needs to be bound to a variable even if it is never used.
        let _rsp = if generate_response_file {
            let temp = response_file(&self.args)
                .context("Failed writing response file")?;

            let mut arg = OsString::new();
            arg.push("@");
            arg.push(&temp);
            cmd.arg(&arg);

            Some(temp)
        } else {
            cmd.args(&self.args);
            None
        };

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(root.join(cwd));
        } else if !root.as_os_str().is_empty() {
            cmd.current_dir(root);
        }

        if let Some(ref env) = self.env {
            cmd.envs(env);
        }

        let output = cmd.output().with_context(|_| {
            format!("Failed to spawn process {:?}", self.program)
        })?;

        // TODO: Interleave stdout and stderr.
        // TODO: Don't buffer stdout/stderr.
        log.write_all(&output.stdout).context("writing stdout")?;
        log.write_all(&output.stderr).context("writing stderr")?;

        if output.status.success() {
            Ok(())
        } else {
            match output.status.code() {
                Some(code) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Process exited with error code {}", code),
                ).into()),
                None => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Process terminated by signal",
                ).into()),
            }
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref display) = self.display {
            write!(f, "{}", display)
        } else {
            write!(f, "{}", Arg::new(self.program.to_string_lossy().as_ref()))?;

            for arg in &self.args {
                write!(f, " {}", Arg::new(arg))?;
            }

            Ok(())
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"")?;

        write!(f, "{}", Arg::new(self.program.to_string_lossy().as_ref()))?;

        for arg in &self.args {
            write!(f, " {}", Arg::new(arg))?;
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

/// Helper type for formatting command line arguments.
struct Arg<'a> {
    arg: &'a str,
}

impl<'a> Arg<'a> {
    pub fn new(arg: &'a str) -> Arg<'a> {
        Arg { arg }
    }

    /// Quotes the argument such that it is safe to pass to the shell.
    #[cfg(windows)]
    pub fn quote(&self, writer: &mut fmt::Write) -> fmt::Result {
        let quote = self.arg.chars().any(|c| c == ' ' || c == '\t')
            || self.arg.is_empty();

        if quote {
            writer.write_char('"')?;
        }

        let mut backslashes: usize = 0;

        for x in self.arg.chars() {
            if x == '\\' {
                backslashes += 1;
            } else {
                // Dump backslashes if we hit a quotation mark.
                if x == '"' {
                    // We need 2n+1 backslashes to escape a quote.
                    for _ in 0..(backslashes + 1) {
                        writer.write_char('\\')?;
                    }
                }

                backslashes = 0;
            }

            writer.write_char(x)?;
        }

        if quote {
            // Escape any trailing backslashes.
            for _ in 0..backslashes {
                writer.write_char('\\')?;
            }

            writer.write_char('"')?;
        }

        Ok(())
    }

    #[cfg(unix)]
    pub fn quote(&self, writer: &mut fmt::Write) -> fmt::Result {
        let quote = self.arg.chars().any(|c| " \n\t#<>'&|".contains(c))
            || self.arg.is_empty();

        if quote {
            writer.write_char('"')?;
        }

        for c in self.arg.chars() {
            // Escape special characters.
            if "\\\"$~".contains(c) {
                writer.write_char('\\')?;
            }

            writer.write_char(c)?;
        }

        if quote {
            writer.write_char('"')?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Arg<'a> {
    /// Converts an argument such that it is safe to append to a command line
    /// string.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.quote(f)
    }
}

/// Writes the response file to a stream.
fn write_response_file<S, I>(args: I, writer: &mut io::Write) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter();

    // Write UTF-8 BOM. Some tools require this to properly decode it as UTF-8
    // instead of ASCII.
    writer.write_all(b"\xEF\xBB\xBF")?;

    if let Some(arg) = iter.next() {
        write!(writer, "{}", Arg::new(arg.as_ref())).unwrap();
    }

    for arg in iter {
        write!(writer, " {}", Arg::new(arg.as_ref())).unwrap();
    }

    // Some programs require a trailing new line (notably LIB.exe and LINK.exe).
    writer.write_all(b"\n")?;

    Ok(())
}

/// Generates a temporary response file for the given command line arguments.
fn response_file<S, I>(args: I) -> io::Result<TempPath>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let tempfile = NamedTempFile::new()?;

    {
        let mut writer = io::BufWriter::new(&tempfile);
        write_response_file(args, &mut writer)?;

        // Explicitly flush to catch any errors.
        writer.flush()?;
    }

    Ok(tempfile.into_temp_path())
}

/// Checks if the given command line arguments are too large and should instead
/// go into a response file. The entire list of arguments, including the program
/// name should be passed to this function.
#[cfg(windows)]
fn args_too_large<S, I>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    use util::Counter;

    // The maximum length is 32768 characters, including the NULL terminator.
    let mut counter = Counter::new();

    let mut iter = args.into_iter();

    if let Some(arg) = iter.next() {
        write!(counter, "{}", Arg::new(arg.as_ref())).unwrap();
    }

    for arg in iter {
        write!(counter, " {}", Arg::new(arg.as_ref())).unwrap();
    }

    // Final NULL terminator.
    counter += 1;

    counter.count() > 32768
}

#[cfg(unix)]
fn args_too_large<S, I>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut size: usize = 0;

    for arg in args {
        // +1 for the NULL terminator.
        size += arg.as_ref().len() + 1;
    }

    // +1 for the final NULL terminator.
    size += 1;

    // Can't be larger than 128 kB.
    size > 0x20000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_arg_display() {
        assert_eq!(format!("{}", Arg::new("foo bar")), "\"foo bar\"");
        assert_eq!(format!("{}", Arg::new("foo\tbar")), "\"foo\tbar\"");
        assert_eq!(format!("{}", Arg::new("foobar")), "foobar");
        assert_eq!(
            format!("{}", Arg::new("\"foo bar\"")),
            "\"\\\"foo bar\\\"\""
        );
        assert_eq!(format!("{}", Arg::new(r"C:\foo\bar")), r"C:\foo\bar");
        assert_eq!(format!("{}", Arg::new(r"\\foo\bar")), r"\\foo\bar");
    }

    #[test]
    #[cfg(unix)]
    fn test_arg_display() {
        assert_eq!(format!("{}", Arg::new("foo bar")), "\"foo bar\"");
        assert_eq!(format!("{}", Arg::new("foo\tbar")), "\"foo\tbar\"");
        assert_eq!(format!("{}", Arg::new("foo\nbar")), "\"foo\nbar\"");
        assert_eq!(format!("{}", Arg::new("foobar")), "foobar");
        assert_eq!(
            format!("{}", Arg::new("\"foo bar\"")),
            "\"\\\"foo bar\\\"\""
        );
        assert_eq!(format!("{}", Arg::new(r"\\foo\bar")), r"\\\\foo\\bar");
        assert_eq!(format!("{}", Arg::new(r"$HOME")), r"\$HOME");
        assert_eq!(format!("{}", Arg::new(r"foo>bar")), "\"foo>bar\"");
        assert_eq!(format!("{}", Arg::new(r"foo&bar")), "\"foo&bar\"");
        assert_eq!(format!("{}", Arg::new(r"~")), r"\~");
        assert_eq!(format!("{}", Arg::new(r"foo|bar")), "\"foo|bar\"");
    }

    #[test]
    fn test_command_display() {
        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo"),
                    vec![String::from("bar"), String::from("baz")]
                )
            ),
            "foo bar baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec![String::from("baz")]
                )
            ),
            "\"foo bar\" baz"
        );

        assert_eq!(
            format!(
                "{}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec![String::from("some argument")]
                ).display(String::from("display this"))
            ),
            "display this"
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo"),
                    vec![String::from("bar"), String::from("baz")]
                )
            ),
            "\"foo bar baz\""
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo bar"),
                    vec![String::from("baz")]
                )
            ),
            "\"\"foo bar\" baz\""
        );

        assert_eq!(
            format!(
                "{:?}",
                Command::new(
                    PathBuf::from("foo/bar/baz"),
                    vec![String::from("some argument")]
                ).display(String::from("display this"))
            ),
            "\"foo/bar/baz \"some argument\"\""
        );
    }
}
