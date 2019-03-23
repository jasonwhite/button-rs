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
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use structopt::StructOpt;

use button::build_graph::{BuildGraph, FromRules};
use button::graph::Graphviz;
use button::rules::Rules;
use button::{BuildState, Error, ResultExt};

use crate::opts::GlobalOpts;
use crate::paths;

#[derive(StructOpt, Debug)]
pub struct Graph {
    /// Path to the build rules. If not specified, finds "button.json" in the
    /// current directory or parent directories.
    #[structopt(long = "rules", short = "r", parse(from_os_str))]
    rules: Option<PathBuf>,

    /// Path to the output file. If not specified, writes to standard output.
    #[structopt(long = "output", short = "o", parse(from_os_str))]
    output: Option<PathBuf>,

    /// Display the cached graph from the previous build. Otherwise, only
    /// displays the build graph derived from the build rules.
    #[structopt(long = "cached")]
    cached: bool,
}

impl Graph {
    /// Shows a pretty graph of the build.
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        let rules = paths::rules_or(self.rules)
            .context("Failed to find build rules")?;

        let build_graph = if self.cached {
            let root = rules.parent().unwrap_or_else(|| Path::new("."));
            let state_path = root.join(paths::STATE);
            let state =
                BuildState::from_path(&state_path).with_context(|_| {
                    format!(
                        "Failed loading build state from '{}'",
                        state_path.display()
                    )
                })?;

            state.graph
        } else {
            let rules = Rules::from_path(&rules).with_context(|_| {
                format!("Failed loading rules from '{}'", rules.display())
            })?;

            BuildGraph::from_rules(rules)?
        };

        if let Some(output) = &self.output {
            let mut stream =
                io::BufWriter::new(fs::File::create(&output).with_context(
                    |_| format!("Failed creating '{}'", output.display()),
                )?);
            build_graph.graphviz(&mut stream)?;
            stream.flush() // Flush to catch write errors
        } else {
            let mut stdout = io::stdout();
            build_graph.graphviz(&mut stdout.lock())?;
            stdout.flush() // Flush to catch write errors
        }
        .context("Failed writing GraphViz DOT file")?;

        Ok(())
    }
}
