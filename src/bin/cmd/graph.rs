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
use std::path::{Path, PathBuf};

use clap;
use num_cpus;

use opts;

/// 'graph' subcommand options
#[derive(Debug)]
pub struct Graph {
    pub file: PathBuf,
    pub threads: usize,
    pub changes: bool,
    pub cached: bool,
    pub full: bool,
    pub edges: opts::Edges,
}

impl Graph {
    pub fn from_matches(matches: &clap::ArgMatches) -> clap::Result<Graph> {
        let cpu_count = num_cpus::get();

        Ok(Graph { file: opts::rules_path(matches.value_of("file")
                                                 .map(Path::new)),
                   threads:
                       value_t!(matches, "threads", usize).unwrap_or(cpu_count),
                   changes: matches.is_present("changes"),
                   cached: matches.is_present("cached"),
                   full: matches.is_present("full"),
                   edges: value_t!(matches.value_of("edges"), opts::Edges)?, })
    }

    /// Shows a pretty graph of your damn software.
    pub fn run(&self) -> i32 {
        println!("{:#?}", self);

        0
    }
}
