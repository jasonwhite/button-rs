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

use cli::opts;

use rules::Rules;
use build::Build;

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
///  3. Diff this graph with the previous *explicit* sub-graph yielding a list
///     of edges and nodes.
///     (a) Merge these changes with the previous (explicit+implicit) graph.
///     (b) Add any *new* resources or tasks to the queue.
///     (c) Delete any *removed* resources. Files should be deleted first,
///         followed by directories. Fail if something cannot be deleted
///         (excluding the case where it doesn't exist). If a directory is not
///         empty, that is a failure. If the build system created a directory,
///         we should have complete control over its contents.
///     (d) If any of the above fails, the new state of the graph should not be
///         committed.
///
///  4. For all resources, check for changes by comparing checksums. Timestamps
///     are unreliable and cannot be used to determine changes and ensure 100%
///     correctness. An option may be added to allow timestamp comparisons to
///     speed things up. However, the default should always be conservative and
///     be the correct option.
///     (a) For any modified output resources, queue the parent task so that it
///         is regenerated. This means an output was modified externally and
///         needs to be brought back to consistency.
///     (b) For any modified input resources, queue that resource.
///
///  5. Walk the graph starting at the queued nodes to create a subgraph. This
///     needs to be done because the graph traversal for the build is not
///     guaranteed to be traversed in the correct order.
pub fn build(opts: opts::Build) -> i32 {

    let path = opts.file.unwrap_or(PathBuf::from("button.json"));
    let root = path.parent().unwrap_or(Path::new("."));

    let build = Build::new(&root, opts.dryrun);

    match Rules::from_path(&path) {
        Ok(rules) => {
            match build.build(&rules) {
                Ok(_) => 0,
                Err(err) => {
                    println!("Error: {}", err);
                    1
                }
            }
        },
        Err(err) => {
            println!("Error: {}", err);
            1
        },
    }
}
