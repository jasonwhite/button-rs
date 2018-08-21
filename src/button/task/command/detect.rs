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

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use os_pipe::PipeReader;
use tempfile::TempPath;

use error::Error;
use util::PathExt;

/// Input and output detection strategy.
#[derive(
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Ord,
    Eq,
    PartialOrd,
    PartialEq,
    Hash,
    Debug,
)]
pub enum Detect {
    /// Detect inputs and outputs for MSVC Cl.exe. This works by adding
    /// `/showIncludes` to the command line and parsing the output.
    Cl,

    /// Don't do any input/output detection. Just run the process as is. This
    /// assumes that all inputs and outputs have been explicitly specified up
    /// front.
    None,
}

impl Default for Detect {
    fn default() -> Self {
        Detect::None
    }
}

impl Detect {
    /// Based on the program name, figure out the best detection method. If not
    /// known, returns `None`.
    #[cfg(windows)]
    pub fn from_program(program: &Path) -> Detect {
        // Only use the file stem on Windows (no file extension).
        if let Some(stem) = program.file_stem() {
            match stem.to_str() {
                Some("cl") => Detect::Cl,
                // Some("gcc") => Detect::GCC,
                // Some("clang") => Detect::Clang,
                _ => Detect::default(),
            }
        } else {
            Detect::None
        }
    }

    /// Based on the program name, figure out the best detection method. If not
    /// known, returns `None`.
    #[cfg(unix)]
    pub fn from_program(program: &Path) -> Detect {
        // Use the whole file name on Unix platforms.
        if let Some(name) = program.file_name() {
            match name.to_str() {
                Some("cl") => Detect::Cl,
                // Some("gcc") => Detect::GCC,
                // Some("clang") => Detect::Clang,
                _ => Detect::None,
            }
        } else {
            Detect::None
        }
    }

    /// Run the given process, returning its inputs and outputs.
    pub fn run(
        &self,
        root: &Path,
        process: Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        match self {
            Detect::Cl => cl::run(root, process, log),
            Detect::None => base::run(root, process, log),
        }
    }
}

/// The sets of detected inputs and outputs of a process.
pub struct Detected {
    inputs: HashSet<PathBuf>,
    outputs: HashSet<PathBuf>,
}

impl Detected {
    pub fn new() -> Detected {
        Detected {
            inputs: HashSet::new(),
            outputs: HashSet::new(),
        }
    }

    pub fn inputs(&self) -> impl Iterator<Item = &Path> {
        self.inputs.iter().map(|p| p.as_ref())
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Path> {
        self.outputs.iter().map(|p| p.as_ref())
    }

    pub fn add_input(&mut self, root: &Path, path: &Path) {
        let path = match path.relative_from(root) {
            Some(x) => x.normalize(),
            None => path.normalize(),
        };

        self.inputs.insert(path);
    }

    pub fn add_output(&mut self, root: &Path, path: &Path) {
        let path = match path.relative_from(root) {
            Some(x) => x.normalize(),
            None => path.normalize(),
        };

        self.outputs.insert(path);
    }
}

pub struct Process {
    child: process::Command,
    reader: PipeReader,
    response_file: Option<TempPath>,
}

impl Process {
    pub fn new(
        child: process::Command,
        reader: PipeReader,
        response_file: Option<TempPath>,
    ) -> Process {
        Process {
            child,
            reader,
            response_file,
        }
    }
}

fn wait_child(mut child: process::Child) -> Result<(), io::Error> {
    let status = child.wait()?;
    match status.code() {
        Some(code) => {
            if code == 0 {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Process exited with error code {}", code),
                ).into())
            }
        }
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;

                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Process terminated by signal {}",
                        status.signal().unwrap()
                    ),
                ).into())
            }

            #[cfg(windows)]
            Ok(())
        }
    }
}

mod base {
    use std::io::{self, Read};
    use std::path::Path;

    use error::{Error, ResultExt};

    use super::{wait_child, Detected, Process};

    pub fn run(
        _root: &Path,
        process: Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let Process {
            mut child,
            mut reader,
            response_file,
        } = process;

        let handle = child.spawn().context("Failed to spawn process")?;
        drop(child);

        let detected = Detected::new();

        // Read the combined stdout/stderr.
        let mut buf = [0u8; 4096];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }

            log.write_all(&buf[0..n])?;
        }

        wait_child(handle)?;

        // NB: The temporary response file needs to outlive the spawned process.
        drop(response_file);

        Ok(detected)
    }
}

mod cl {
    use std::io::{self, BufRead};
    use std::path::Path;

    use error::{Error, ResultExt};

    use super::{wait_child, Detected, Process};

    static INCLUDE_PREFIX: &str = "Note: including file: ";

    pub fn run(
        root: &Path,
        process: Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let Process {
            mut child,
            reader,
            response_file,
        } = process;

        let mut reader = io::BufReader::new(reader);

        child.arg("/showIncludes");

        let handle = child.spawn().context("Failed to spawn process")?;
        drop(child);

        let mut detected = Detected::new();

        let mut line = String::new();

        while reader.read_line(&mut line)? != 0 {
            if line.starts_with(INCLUDE_PREFIX) {
                let include = &line[INCLUDE_PREFIX.len()..].trim();
                detected.add_input(root, Path::new(include));
            } else {
                log.write_all(line.as_ref())?;
            }

            line.clear();
        }

        wait_child(handle)?;

        drop(response_file);

        Ok(detected)
    }
}
