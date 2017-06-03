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

use cli::opts;

/// 'clean' subcommand options
#[derive(Debug)]
pub struct Clean {
    pub file: PathBuf,
    pub dryrun: bool,
    pub color: opts::Coloring,
    pub threads: usize,
    pub purge: bool,
}

impl Clean {
    pub fn from_matches(matches: &clap::ArgMatches) -> clap::Result<Clean> {
        let cpu_count = num_cpus::get();

        Ok(Clean {
               file: opts::rules_path(matches.value_of("file").map(Path::new)),
               dryrun: matches.is_present("dryrun"),
               color: value_t!(matches.value_of("color"), opts::Coloring)?,
               threads: value_t!(matches, "threads", usize).unwrap_or(cpu_count),
               purge: matches.is_present("purge"),
           })
    }

    /// Cleans your damn software.
    pub fn run(&self) -> i32 {
        println!("{:#?}", self);

        0
    }
}
