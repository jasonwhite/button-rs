// Copyright (c) 2019 Jason White
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
use std::io;
use std::path::{Path, PathBuf};

use button::build_graph::{Edge, Node};
use button::graph::{Indexable, Neighbors, Nodes};
use button::{res, task, BuildState, Error, ResultExt};

use serde_json as json;

use crate::opts::GlobalOpts;
use crate::paths;

#[derive(Serialize, Deserialize)]
struct CompleteRule {
    /// Explicit inputs to the task.
    pub inputs: res::Set,

    /// Implicit inputs to the task.
    pub inputs_implicit: res::Set,

    /// Implicit inputs to the task.
    pub outputs: res::Set,

    /// Implicit inputs to the task.
    pub outputs_implicit: res::Set,

    /// The sequence of tasks to execute.
    pub tasks: task::List,
}

#[derive(StructOpt, Debug)]
pub struct Dump {
    /// Path to the build rules. If not specified, finds "button.json" in the
    /// current directory or parent directories.
    #[structopt(long = "rules", short = "r", parse(from_os_str))]
    rules: Option<PathBuf>,
}

impl Dump {
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        let rules = paths::rules_or(self.rules)
            .context("Failed to find build rules")?;

        let root = rules.parent().unwrap_or_else(|| Path::new("."));

        let state_path = root.join(paths::STATE);

        let graph = BuildState::from_path(state_path)?.graph;

        let mut rules = Vec::new();

        for index in graph.nodes() {
            match graph.node_from_index(index) {
                Node::Task(t) => {
                    let mut inputs = res::Set::new();
                    let mut inputs_implicit = res::Set::new();
                    let mut outputs = res::Set::new();
                    let mut outputs_implicit = res::Set::new();

                    for (incoming, edge) in graph.incoming(index) {
                        let incoming = graph.node_from_index(incoming).as_res();
                        match graph.edge_from_index(edge).1 {
                            Edge::Explicit => {
                                inputs.insert(incoming.clone());
                            }
                            Edge::Implicit => {
                                inputs_implicit.insert(incoming.clone());
                            }
                        }
                    }

                    for (outgoing, edge) in graph.outgoing(index) {
                        let outgoing = graph.node_from_index(outgoing).as_res();
                        match graph.edge_from_index(edge).1 {
                            Edge::Explicit => {
                                outputs.insert(outgoing.clone());
                            }
                            Edge::Implicit => {
                                outputs_implicit.insert(outgoing.clone());
                            }
                        }
                    }

                    let rule = CompleteRule {
                        inputs,
                        inputs_implicit,
                        outputs,
                        outputs_implicit,
                        tasks: t.clone(),
                    };

                    rules.push(rule);
                }
                Node::Resource(_) => {}
            }
        }

        json::to_writer_pretty(io::stdout().lock(), &rules)
            .context("Failed serializing build rules")?;

        Ok(())
    }
}

// Possible queries:
//  - What are the transitive outputs if this file is modified?
//    * Involves BFS/DFS
//  - What are the transitive dependencies of this file?
//    * Involves BFS/DFS
//  - What tasks is this file an input to?
