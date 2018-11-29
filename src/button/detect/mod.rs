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

pub mod base;
pub mod cl;
pub mod clang;
mod detected;

pub use self::detected::Detected;

use std::io;
use std::path::Path;

use error::Error;

use util::Process;

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

    /// Detect inputs and outputs for Clang or GCC. This works by adding the
    /// `-MMD -MF temp.d` flags to the compilation command line and parsing the
    /// resulting Makefile. Note that we do not care about system header files
    /// here, hence the `-MMD` flag instead of `-MD`.
    Clang,

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
                Some("gcc") => Detect::Clang,
                Some("clang") => Detect::Clang,
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
                Some("gcc") => Detect::Clang,
                Some("clang") => Detect::Clang,
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
            Detect::Clang => clang::run(root, process, log),
            Detect::None => base::run(root, process, log),
        }
    }
}
