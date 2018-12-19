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
use std::io::Write as IoWrite;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use tempfile;

use crate::error::Error;

use super::traits::Task;
use crate::detect::Detected;

use crate::util::{progress_dummy, Arguments, Process, Retry};

/// A task to create a directory.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
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
    retry: Option<Retry>,
}

impl BatchScript {
    fn execute_impl(
        &self,
        root: &Path,
        log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        // Write the script contents to a temporary file for execution. This
        // temporary file must outlive the spawned process.
        let temppath = {
            let mut tmp = tempfile::Builder::new().suffix(".bat").tempfile()?;
            tmp.as_file_mut().write_all(self.contents.as_bytes())?;
            tmp.into_temp_path()
        };

        let mut args = Arguments::new();

        if self.quiet {
            args.push("/Q".into());
        }

        args.push("/c".into());
        args.push("call".into());
        args.push(temppath.to_str().unwrap().into());

        let mut process = Process::new(PathBuf::from("cmd.exe"), args);
        process.env = self.env.clone();

        let (mut reader, child) = process.spawn(root)?;

        let mut buf = [0u8; 4096];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }

            log.write_all(&buf[0..n])?;
        }

        child.wait()?;

        // Don't do dependency detection for now. We may want to use generic
        // dependency detection in this case.
        Ok(Detected::new())
    }
}

impl fmt::Display for BatchScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display {
            Some(ref display) => write!(f, "batch script: {}", display),
            None => write!(f, "batch script"),
        }
    }
}

impl fmt::Debug for BatchScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for BatchScript {
    fn execute(
        &self,
        root: &Path,
        log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        if let Some(ref retry) = self.retry {
            retry.call(|| self.execute_impl(root, log), progress_dummy)
        } else {
            self.execute_impl(root, log)
        }
    }
}
