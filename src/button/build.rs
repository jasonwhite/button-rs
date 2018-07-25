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

use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use build_graph::{BuildGraph, Node};
use res::{self, Resource};
use rules::Rules;
use task::{self, Task};

use failure::Error;

/// A build failure. Contains each of the node indexes that failed and the
/// associated error.
#[derive(Fail, Debug)]
pub struct BuildFailure {
    errors: Vec<(usize, Error)>,
}

impl BuildFailure {
    pub fn new(errors: Vec<(usize, Error)>) -> BuildFailure {
        BuildFailure { errors }
    }
}

impl fmt::Display for BuildFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.errors.len() == 1 {
            write!(f, "Build failed with {} error", self.errors.len())
        } else {
            write!(f, "Build failed with {} errors", self.errors.len())
        }
    }
}

/// Represents a build. This holds the context necessary for all build
/// operations.
pub struct Build<'a> {
    /// Root of the build. This is the directory containing the "button.json"
    /// file and is the default path from which all subprocesses are spawned.
    /// The working directories of tasks are relative to this path.
    root: &'a Path,

    /// Whether or not this is a dry run.
    dryrun: bool,
}

impl<'a> Build<'a> {
    pub fn new(root: &'a Path, dryrun: bool) -> Build<'a> {
        Build { root, dryrun }
    }

    /// Runs a build.
    pub fn build(&self, rules: Rules, threads: usize) -> Result<(), Error> {
        use graph::Algo;

        if self.dryrun {
            println!("Note: This is a dry run. Nothing is affected.");
        }

        let graph = BuildGraph::from_rules(rules)?;

        #[cfg(target_os = "TODO")]
        {
            let (state, removed) =
                self.state.update(BuildGraph::from_rules(rules)?);
            if !removed.is_empty() {
                // Delete removed nodes in reverse topological order.
                //
                // TODO: Construct a subgraph that only includes the removed
                // nodes!
                state.graph.traverse(
                    |id, node| self.visit_delete(id, node),
                    threads,
                    true,
                )?;
            }
        }

        // Traverse the graph, building everything in topological order.
        match graph.traverse(|id, node| self.visit(id, node), threads, false) {
            Ok(()) => Ok(()),
            Err(errors) => {
                // TODO: For each node that failed to build, add it back to the
                // queue so it gets executed again next time the build is run.

                Err(BuildFailure::new(errors).into())
            }
        }
    }

    /// Visitor function for a node.
    fn visit(&self, id: usize, node: &Node) -> Result<bool, Error> {
        match node {
            Node::Resource(r) => self.visit_resource(id, r),
            Node::Task(t) => self.visit_task(id, t),
        }
    }

    /// Called when visiting a resource type node in the build graph.
    fn visit_resource(
        &self,
        _id: usize,
        _node: &res::Any,
    ) -> Result<bool, Error> {
        // println!("thread {} :: {}", id, node);

        // TODO: Determine if this resource has changed.

        // Only visit child nodes if this node's state has changed. For example,
        // when compiling an object file, if the generated object file has not
        // changed, there is no need to perform linking.
        Ok(true)
    }

    /// Called when visiting a task type node in the build graph.
    fn visit_task(&self, id: usize, node: &task::List) -> Result<bool, Error> {
        let mut stdout = io::stdout();

        let mut output = Vec::new();

        for task in node.iter() {
            writeln!(output, "[{}] {}", id, task)?;

            if !self.dryrun {
                match task.execute(self.root, &mut output) {
                    Err(err) => {
                        writeln!(output, "Error: {}", err)?;
                        stdout.write_all(&output)?;
                        return Err(err);
                    }
                    Ok(()) => {}
                };
            }
        }

        stdout.write_all(&output)?;

        Ok(true)
    }

    /// Called when a node is deleted from the graph.
    #[allow(dead_code)]
    fn visit_delete(&self, _id: usize, node: &Node) -> Result<bool, Error> {
        match node {
            Node::Resource(r) => {
                if !self.dryrun {
                    r.delete()?;
                }
            }
            Node::Task(_) => {
                // It doesn't make sense for tasks to be deleted.
            }
        }

        Ok(true)
    }
}
