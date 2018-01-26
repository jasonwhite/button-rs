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

use std::path::Path;

use std::io::{self, Write};

use std::error::Error;

use error;
use graph;
use rules::Rules;
use res;
use task::{self, Task};

/// Represents a build. This holds the context necessary for all build
/// operations.
pub struct Build<'a> {
    /// Root of the build. This is the directory containing the "button.json"
    /// file and is the default path from which all subprocesses are spawned.
    /// The working directories of tasks are relative to this path.
    root: &'a Path,

    /// Whether or not this is a dry run. This needs to be passed to child build
    /// systems.
    dryrun: bool,
}

impl<'a> Build<'a> {
    pub fn new(root: &'a Path, dryrun: bool) -> Build<'a> {
        Build {
            root: root,
            dryrun: dryrun,
        }
    }

    /// Runs a build.
    pub fn build<'b>(&self,
                     rules: &'b Rules,
                     threads: usize)
                     -> Result<(), error::Error<'b>> {

        println!("Root directory: {:?}", self.root);

        if self.dryrun {
            println!("Note: This is a dry run. Nothing is affected.");
        }

        let g = graph::from_rules(rules)?;

        if let Err(errors) = graph::traverse(&g,
                                          |id, node| self.visit(id, node),
                                          threads) {
            // TODO: Propagate errors.
            println!("The following errors occurred during the build:");
            for err in errors {
                println!("{:?}", err);
            }
        }

        Ok(())
    }

    /// Visitor function for a node.
    fn visit(&self, id: usize, node: graph::Node) -> Result<bool, io::Error> {
        match node {
            graph::Node::Resource(r) => self.visit_resource(id, r),
            graph::Node::Task(t) => self.visit_task(id, t),
        }
    }

    /// Called when visiting a resource type node in the build graph.
    fn visit_resource(&self,
                      _id: usize,
                      _node: &res::Any)
                      -> Result<bool, io::Error> {
        //println!("thread {} :: {}", id, node);

        // TODO: Determine if this resource has changed.

        // Only visit child nodes if this node's state has changed. For example,
        // when compiling an object file, if the generated object file has not
        // changed, there is no need to perform linking.
        Ok(true)
    }

    /// Called when visiting a task type node in the build graph.
    fn visit_task(&self,
                  id: usize,
                  node: &task::List)
                  -> Result<bool, io::Error> {

        let mut stdout = io::stdout();

        let mut output = Vec::new();

        for task in node.iter() {
            writeln!(output, "[{}] {}", id, task)?;

            match task.execute(&mut output) {
                Err(err) => {
                    writeln!(output, "Error: {}", err.description())?;
                    stdout.write(&output)?;
                    return Err(err);
                }
                Ok(()) => {}
            };
        }

        stdout.write(&output)?;

        Ok(true)
    }
}
