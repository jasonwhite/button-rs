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

use std::ffi::OsStr;
use std::io::{self, BufRead};
use std::path::Path;

use crate::error::{Error, ResultExt};
use crate::util::Process;

use super::detected::Detected;

static INCLUDE_PREFIX: &str = "Note: including file: ";

pub fn run(
    root: &Path,
    process: &Process,
    log: &mut dyn io::Write,
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
                if path.extension() == Some(OsStr::new(".tlh")) {
                    // TLH includes are a special case. These are actually
                    // outputs of the preprocessing step. An `#import <foo.tlb>`
                    // will generate a `foo.tlh` file in the output directory
                    // and perform an `#include` on it. If we don't track these
                    // outputs, they won't get cleaned up correctly. Tracking
                    // this also helps detect race conditions. Two compilation
                    // steps should not generate the same TLH file. To avoid
                    // that problem, it is best to restrict `#import` directives
                    // to only precompiled headers.
                    detected.add_output(path.into());
                } else {
                    detected.add_input(path.into());
                }
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
