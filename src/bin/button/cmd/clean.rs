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

use num_cpus;
use failure::Error;

use button::{self, logger};

use opts::GlobalOpts;
use paths;

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// Path to the build rules. If not specified, finds "button.json" in the
    /// current directory or parent directories.
    #[structopt(long = "rules", short = "r", parse(from_os_str))]
    rules: Option<PathBuf>,

    /// Doesn't delete anything. Just prints what would be deleted.
    #[structopt(long = "dryrun", short = "n")]
    dryrun: bool,

    /// Print additional information.
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,

    /// The number of threads to use. Defaults to the number of logical cores.
    #[structopt(short = "t", long = "threads", default_value = "0")]
    threads: usize,
}

impl Clean {
    pub fn main(&self, global: &GlobalOpts) -> Result<(), Error> {
        let rules = paths::rules_path(&self.rules);
        let threads = if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        };

        let root = rules.parent().unwrap_or_else(|| Path::new("."));

        let logger = logger::Console::new(self.verbose, global.color.into());

        let build = button::Build::new(root, Path::new(paths::STATE));
        build.clean(self.dryrun, threads, &logger)
    }
}
