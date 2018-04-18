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

use std::str::FromStr;

use std::env;
use std::path::{Path, PathBuf};

/// Coloring of command output.
#[derive(Debug)]
pub enum Coloring {
    Auto,
    Never,
    Always,
}

impl FromStr for Coloring {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Coloring::Auto),
            "never" => Ok(Coloring::Never),
            "always" => Ok(Coloring::Always),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Edges {
    Explicit,
    Implicit,
    Both,
}

impl FromStr for Edges {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "explicit" => Ok(Edges::Explicit),
            "implicit" => Ok(Edges::Implicit),
            "both" => Ok(Edges::Both),
            _ => Err(()),
        }
    }
}

/// Returns an absolute path to the rules, starting at the given directory. The
/// canonical name for the JSON rules file is "button.json". This function shall
/// search for the file in the given starting directory and all parent
/// directories. Returns `None` if it cannot be found.
pub fn find_rules_path(start: &Path) -> Option<PathBuf> {
    let path = start.join("button.json");

    if path.is_file() {
        Some(path)
    } else {
        // Search in the parent directory.
        match start.parent() {
            Some(parent) => find_rules_path(parent),
            None => None,
        }
    }
}

/// Returns a path to the rules.
pub fn rules_path(path: Option<&Path>) -> PathBuf {
    match path {
        Some(path) => path.to_path_buf(),
        None => {
            let cwd = env::current_dir().unwrap();
            match find_rules_path(&cwd) {
                Some(path) => path,

                // Not found. Just assume it lives in the current directory even
                // though it doesn't (or it would have been found). The error
                // will get reported when trying to load this file later.
                None => PathBuf::from("button.json"),
            }
        }
    }
}
