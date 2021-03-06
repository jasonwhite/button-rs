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
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use num_cpus;
use structopt::StructOpt;

use button::{self, events, Error, ResultExt, Rules};

use crate::opts::GlobalOpts;
use crate::paths;

#[derive(StructOpt, Debug)]
pub struct Build {
    /// Path to the build rules. If not specified, finds "button.json" in the
    /// current directory or parent directories.
    #[structopt(long = "rules", short = "r", parse(from_os_str))]
    rules: Option<PathBuf>,

    /// Doesn't run the build. Just prints the tasks that will be executed.
    #[structopt(short = "n", long = "dryrun")]
    dryrun: bool,

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
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        let rules = paths::rules_or(self.rules)
            .context("Failed to find build rules")?;

        let threads = if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        };

        let root = rules.parent().unwrap_or_else(|| Path::new("."));

        // Ensure the .button directory exists.
        paths::init(&root).context("Failed initializing .button directory")?;

        let f = fs::File::create(paths::LOG)
            .with_context(|_| format!("Failed to create '{}'", paths::LOG))?;

        // TODO: If stderr is not a TTY, use an event handler that dumps the
        // full output to a file.
        let event_handler: Vec<events::AnyHandler> =
            vec![events::Console::new().into(), events::Binary::new(f).into()];
        let (sender, receiver) = mpsc::channel();
        let _event_thread = events::EventThread::new(event_handler, receiver);

        let state_path = root.join(paths::STATE);
        let build = button::Build::new(root, &state_path, threads, sender);

        if self.clean {
            build.clean(self.dryrun)?;
        }

        let rules = Rules::from_path(&rules)?;

        build.build(rules, self.dryrun)?;

        Ok(())
    }
}
