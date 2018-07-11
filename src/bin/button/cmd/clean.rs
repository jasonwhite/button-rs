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
use std::path::PathBuf;

use opts::Coloring;

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// Path to the build description. If not specified, finds "button.json"
    /// in the current directory or parent directories.
    #[structopt(long = "file", short = "s", parse(from_os_str))]
    file: Option<PathBuf>,

    /// Doesn't delete anything. Just prints what would be deleted.
    #[structopt(long = "dryrun", short = "n")]
    dryrun: bool,

    /// Print additional information.
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,

    /// When to colorize the output.
    #[structopt(long = "color")]
    color: Coloring,

    /// Deletes the build state too.
    #[structopt(long = "purge")]
    purge: bool,
}

impl Clean {
    pub fn main(&self) -> i32 {
        println!("{:#?}", self);

        0
    }
}
