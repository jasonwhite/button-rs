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

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::{Error, ResultExt};
use crate::util::Process;

use super::detected::Detected;

use tempfile::NamedTempFile;

use crate::util::MakeFile;

pub fn run(
    root: &Path,
    process: &Process,
    log: &mut io::Write,
) -> Result<Detected, Error> {
    let mut process = process.clone();

    // Use `-MMD -MF` to capture header files used by the build.
    //
    // TODO: Handle the case where this is already in the command line arguments
    // or when we are not compiling (i.e., no `-c` flag).
    process.args.push("-MMD".into());
    process.args.push("-MF".into());

    let temppath = NamedTempFile::new()
        .context("Failed creating temporary deps file")?
        .into_temp_path();
    process
        .args
        .push(temppath.to_string_lossy().into_owned().into());

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

    let mut makefile = fs::read_to_string(&temppath).with_context(|_| {
        format!("Failed reading temporary file \"{}\"", temppath.display())
    })?;

    // The makefile parser requires a null terminator.
    makefile.push('\0');

    // TODO: Fix error handling!
    let makefile = MakeFile::from_str(&makefile).unwrap();

    let mut detected = Detected::new();

    for rule in makefile.rules() {
        for input in &rule.prereqs {
            // TODO: Handle working directories and whatnot.
            detected.add_input(PathBuf::from(input).into());
        }
    }

    // Detect errors in deleting temporary file.
    temppath
        .close()
        .context("Failed deleting temporary deps file")?;

    Ok(detected)
}
