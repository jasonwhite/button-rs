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

use button::util::PathExt;
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
/// Returns `None` if it cannot be found.
pub fn find_rules_path(start: &Path) -> Option<PathBuf> {
    let path = start.join(RULES);

    if path.is_file() {
        // Path was found. Return a path relative to `start`.
        Some(
            path.relative_from(&env::current_dir().unwrap())
                .unwrap_or(path),
        )
    } else {
        // Search in the parent directory.
        match start.parent() {
            Some(parent) => find_rules_path(parent),
            None => None,
        }
    }
}

/// Returns a path to the rules.
pub fn rules_path(path: &Option<PathBuf>) -> PathBuf {
    match path {
        Some(ref path) => path.to_path_buf(),
        None => {
            let cwd = env::current_dir().unwrap();
            match find_rules_path(&cwd) {
                Some(path) => path,

                // Not found. Just assume it lives in the current directory even
                // though it doesn't (or it would have been found). The error
                // will get reported when trying to load this file later.
                None => PathBuf::from(RULES),
            }
        }
    }
}

/// Initializes the .button directory.
///
/// Nothing is done if it already exists.
pub fn init(root: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(root.join(DIR))
}
