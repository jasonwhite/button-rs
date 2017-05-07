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
use std::path::PathBuf;

/// Coloring of command output.
#[derive(Debug)]
pub enum Coloring {
    Auto,
    Never,
    Always
}

impl FromStr for Coloring {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto"   => Ok(Coloring::Auto),
            "never"  => Ok(Coloring::Never),
            "always" => Ok(Coloring::Always),
            _        => Err("no match")
        }
    }
}

#[derive(Debug)]
pub enum Edges {
    Explicit,
    Implicit,
    Both
}

impl FromStr for Edges {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "explicit" => Ok(Edges::Explicit),
            "implicit" => Ok(Edges::Implicit),
            "both"     => Ok(Edges::Both),
            _          => Err(())
        }
    }
}

/// 'build' subcommand options
#[derive(Debug)]
pub struct Build {
    pub file: Option<PathBuf>,
    pub dryrun: bool,
    pub color: Coloring,
    pub threads: usize,
    pub autobuild: bool,
    pub delay: usize,
}

/// 'clean' subcommand options
#[derive(Debug)]
pub struct Clean {
    pub file: Option<PathBuf>,
    pub dryrun: bool,
    pub color: Coloring,
    pub threads: usize,
    pub purge: bool,
}

/// 'graph' subcommand options
#[derive(Debug)]
pub struct Graph {
    pub file: Option<PathBuf>,
    pub threads: usize,
    pub changes: bool,
    pub cached: bool,
    pub full: bool,
    pub edges: Edges,
}
