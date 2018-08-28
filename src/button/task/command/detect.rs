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

use std::io;
use std::path::Path;

use error::Error;

use util::Process;

use task::traits::Detected;

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
        self,
        root: &Path,
        process: &Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        match self {
            Detect::Cl => cl::run(root, process, log),
            Detect::None => base::run(root, process, log),
        }
    }
}

mod base {
    use std::borrow::Cow;
    use std::io::{self, Read};
    use std::path::Path;

    use error::{Error, ResultExt};
    use util::Process;

    use task::traits::Detected;

    pub fn run(
        root: &Path,
        process: &Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let mut process = Cow::Borrowed(process);

        // Generate a response file if necessary.
        let response_file = if process.args.too_large() {
            Some(
                process
                    .to_mut()
                    .response_file()
                    .context("Failed generating response file")?,
            )
        } else {
            None
        };

        let (mut reader, child) = process.spawn(root)?;

        // Read the combined stdout/stderr.
        let mut buf = [0u8; 4096];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }

            log.write_all(&buf[0..n])?;
        }

        child.wait()?;

        if let Some(response_file) = response_file {
            response_file
                .close()
                .context("Failed deleting response file")?;
        }

        Ok(Detected::new())
    }
}

mod cl {
    use std::io::{self, BufRead};
    use std::path::Path;

    use error::{Error, ResultExt};
    use util::Process;

    use task::traits::Detected;

    static INCLUDE_PREFIX: &str = "Note: including file: ";

    pub fn run(
        root: &Path,
        process: &Process,
        log: &mut io::Write,
    ) -> Result<Detected, Error> {
        let mut process = process.clone();

        // Use `/showIncludes` to capture header files that are used by the
        // build.
        //
        // TODO: Use the `VS_UNICODE_OUTPUT` environment variable to get Unicode
        // output from cl.exe.
        //
        // TODO: Echo "Note: including file: " line if "/showIncludes" is
        // already present in the command line arguments. This should
        // handle the various formats of "/showIncludes" as well (e.g.,
        // starting with '-' instead of '/', case differences).
        process.args.push("/showIncludes".into());

        // Generate a response file if necessary.
        let response_file = if process.args.too_large() {
            Some(
                process
                    .response_file()
                    .context("Failed generating response file")?,
            )
        } else {
            None
        };

        let (reader, child) = process.spawn(root)?;

        // Canonicalize the root path such that `strip_prefix` works below.
        let root = root.canonicalize()?;

        // Buffer the read such that we can read one line at a time.
        let mut reader = io::BufReader::new(reader);

        let mut detected = Detected::new();

        let mut line = String::new();

        while reader.read_line(&mut line)? != 0 {
            if line.starts_with(INCLUDE_PREFIX) {
                let include = &line[INCLUDE_PREFIX.len()..].trim();

                // Canonicalize the path such that the root path and this path
                // agree on the case of the file path. Otherwise, `strip_prefix`
                // won't work.
                let path = Path::new(include).canonicalize()?;

                // Only include paths that are contained within the project
                // root. Everything else is treated as a system dependency.
                if let Ok(path) = path.strip_prefix(&root) {
                    detected.add_input(path.to_path_buf().into());
                }
            } else {
                log.write_all(line.as_ref())?;
            }

            line.clear();
        }

        child.wait()?;

        if let Some(response_file) = response_file {
            response_file
                .close()
                .context("Failed deleting response file")?;
        }

        Ok(detected)
    }
}
