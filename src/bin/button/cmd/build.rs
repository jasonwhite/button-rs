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

use button::{build, logger, Rules};

use opts::{rules_path, Coloring};

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
            possible_values = "&Coloring::variants()",
            case_insensitive = "true"
        )
    )]
    color: Coloring,

    /// The number of threads to use. Defaults to the number of logical cores.
    #[structopt(short = "t", long = "threads", default_value = "0")]
    threads: usize,

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
    /// Runs an incremental build.
    ///
    /// The build algorithm proceeds as follows:
    ///
    ///  1. Read the JSON build description into a sequence of rules.
    ///
    ///  2. Convert it to a bipartite graph.
    ///     (a) Fails if cycles exist.
    ///     (b) Fails if race condition exists.
    ///
    ///  3. Diff this graph with the previous *explicit* sub-graph yielding a
    ///     list of edges and nodes.
    ///     (a) Merge these changes with the previous (explicit+implicit) graph.
    ///     (b) Add any *new* resources or tasks to the queue.
    ///     (c) Delete any *removed* resources. Files should be deleted first,
    ///         followed by directories. Fail if something cannot be deleted
    ///         (excluding the case where it doesn't exist). If a directory is
    ///         not empty, that is a failure. If the build system created a
    ///         directory, we should have complete control over its contents.
    ///     (d) If any of the above fails, the new state of the graph should not
    ///         be committed.
    ///
    ///  4. For all resources, check for changes by comparing checksums.
    ///     Timestamps are unreliable and cannot be used to determine changes
    ///     and ensure 100% correctness. An option may be added to allow
    ///     timestamp comparisons to speed things up. However, the default
    ///     should always be conservative and be the correct option.
    ///     (a) For any modified output resources, queue the parent task so that
    ///         it is regenerated. This means an output was modified externally
    ///         and needs to be brought back to consistency.
    ///     (b) For any modified input resources, queue that resource.
    ///
    ///  5. Walk the graph starting at the queued nodes to create a subgraph.
    ///     This needs to be done because the graph traversal for the build is
    ///     not guaranteed to be traversed in the correct order.
    pub fn main(&self) -> Result<(), Error> {
        let file = rules_path(&self.file);
        let threads = if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads
        };

        let root = file.parent().unwrap_or_else(|| Path::new("."));

        let rules = Rules::from_path(&file).with_context(|_err| {
            format!("Failed loading rules from file {:?}", file)
        })?;

        // TODO: Pass the verbosity setting to the logger.
        let logger = logger::Console::new();

        build(root, rules, self.dryrun, threads, logger)
    }
}
