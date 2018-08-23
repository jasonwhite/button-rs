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

//! Well-known paths for the build system.
//!
//! All paths are relative to the project root (i.e, the directory that
//! `button.json` lives in).
//!
//! Note that the button library doesn't have these constants hard coded for a
//! reason. The locations of these files are entirely up to the library user to
//! configure.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Name of the rules file.
pub const RULES: &str = "button.json";

/// Name of the directory where internal state can be stored.
pub const DIR: &str = ".button";

/// Name of the file where build state is saved. This lives inside the ".button"
/// directory.
pub const STATE: &str = ".button/state";

/// Name of the file where the last build log is stored.
pub const LOG: &str = ".button/log";

/// Returns a path to the rules, starting at the given directory. The canonical
/// name for the JSON rules file is "button.json". This function shall search
/// for the file in the given starting directory and all parent directories.
/// Returns an error if it cannot be found.
pub fn find_rules(mut path: PathBuf) -> io::Result<PathBuf> {
    loop {
        path.push(RULES);

        match path.metadata() {
            Ok(metadata) => {
                if metadata.is_file() {
                    return Ok(path);
                }
            }
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => (),
                _ => return Err(err),
            },
        };

        // Pop the added path and the parent directory, then try again.
        if !path.pop() || !path.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Reached file system root looking for '{}'", RULES),
            ));
        }
    }
}

/// If a rules path is given, returns the absolute path to them. Otherwise,
/// searches through parent directories to find the rules path.
///
/// This always returns an absolute path to the rules.
pub fn rules_or(path: Option<PathBuf>) -> io::Result<PathBuf> {
    let mut cwd = env::current_dir()?;

    match path {
        Some(path) => {
            cwd.push(path);
            Ok(cwd)
        }
        None => find_rules(cwd),
    }
}

/// Initializes the .button directory.
///
/// Nothing is done if it already exists.
pub fn init(root: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(root.join(DIR))
}
