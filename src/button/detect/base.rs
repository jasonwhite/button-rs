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

use std::borrow::Cow;
use std::io::{self, Read};
use std::path::Path;

use error::{Error, ResultExt};
use util::Process;

use super::detected::Detected;

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
