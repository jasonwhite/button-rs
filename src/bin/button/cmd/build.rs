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

use button::{build, clean, logger, Rules};

use opts::{rules_path, ColorChoice};

use failure::{Error, ResultExt};

#[derive(StructOpt, Debug)]
pub struct Build {
    /// Path to the build description. If not specified, finds "button.json" in
    /// the current directory or parent directories.
    #[structopt(long = "file", short = "f", parse(from_os_str))]
    file: Option<PathBuf>,

    /// Doesn't run the build. Just prints the tasks that will be executed.
    #[structopt(short = "n", long = "dryrun")]
    dryrun: bool,

    /// Print additional information.
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,

    /// When to colorize the output.
    #[structopt(
        long = "color",
        default_value = "auto",
        raw(
            possible_values = "&ColorChoice::variants()",
            case_insensitive = "true"
        )
    )]
    color: ColorChoice,

    /// The number of threads to use. Defaults to the number of logical cores.
    #[structopt(short = "t", long = "threads", default_value = "0")]
    threads: usize,

    /// Deletes all generated files before building. This is equivalent to
    /// running `button clean` first.
    #[structopt(long = "clean")]
    clean: bool,

    /// Watch for changes and build automatically.
    #[structopt(long = "watch")]
    watch: bool,

    /// Used with "--watch". The directory to watch for changes in. Useful when
    /// building inside a FUSE file system. Defaults to the current working
    /// directory.
    #[structopt(long = "watch-dir", parse(from_os_str))]
    watch_dir: Option<PathBuf>,

    /// Used with "--watch". The number of milliseconds to wait before
    /// building. The timeout is reset every time a new change is seen.
    /// Useful when source code is changed by a tool (e.g., git checkout,
    /// automatic formatting, etc).
    #[structopt(long = "watch-delay", default_value = "100")]
    watch_delay: usize,
}

impl Build {
    pub fn main(&self) -> Result<(), Error> {
        let file = rules_path(&self.file);
        let threads = if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        };

        let root = file.parent().unwrap_or_else(|| Path::new("."));

        // Log to both the console and a binary file for later analysis.
        //
        // TODO: Write the binary log to .button/log and have `button replay`
        // automatically replay it if no file is specified.
        let mut loggers = logger::LoggerList::new();
        loggers
            .push(logger::Console::new(self.verbose, self.color.into()).into());
        loggers.push(logger::binary_file_logger("build.log")?.into());

        if self.clean {
            clean(root, self.dryrun, threads, &loggers)?;
        }

        let rules = Rules::from_path(&file).with_context(|_err| {
            format!("Failed loading rules from file {:?}", file)
        })?;

        build(root, rules, self.dryrun, threads, &mut loggers)
    }
}
