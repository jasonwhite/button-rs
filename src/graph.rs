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

use std::error;
use std::fmt;

use petgraph::algo::tarjan_scc;
use petgraph::graphmap;

use resources;
use tasks;
use rules::Rules;

#[derive(Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Edge {
    /// An explicit edge is one that is user-defined in the build description.
    /// That is, it is *explicitly* declared.
    Explicit,

    /// An implicit edge is one that is automatically determined after the task
    /// is executed. That is, it is *implicitly* discovered. Tasks, when
    /// executed, return resources that are read from or written to. The edges
    /// associated with these resources are then implicit. It is usually the
    /// case that, for every implicit edge, there is an equivalent explicit
    /// edge.
    Implicit,
}

/// A node in the graph.
#[derive(Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Node<'a> {
    Resource(&'a resources::File),
    Task(&'a Vec<tasks::Command>),
}

impl<'a> fmt::Display for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Resource(ref r) => write!(f, "resource: {}", r),
            Node::Task(ref t) => write!(f, "task: {:?}", t),
        }
    }
}

/// The build graph.
pub type BuildGraph<'a> = graphmap::DiGraphMap<Node<'a>, Edge>;

/// A cycle in the graph. A cycle is denoted by the nodes contained in the
/// cycle. The nodes in the cycle should be in topological order. That is, each
/// node's parent must be the previous node in the list.
#[derive(Debug)]
pub struct Cycle<'a> {
    nodes: Vec<Node<'a>>,
}

impl<'a> fmt::Display for Cycle<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut it = self.nodes.iter();

        // Unwrapping because there must always be at least one node in a cycle.
        // If this panics, then the code creating the cycle is buggy.
        let first = it.next().unwrap();

        write!(f, "    {}\n", first)?;

        for node in it {
            write!(f, " -> {}\n", node)?;
        }

        // Make the cycle obvious
        write!(f, " -> {}\n", first)
    }
}

/// Error for when one or more cycles are detected in the build graph.
#[derive(Debug)]
pub struct CyclesError<'a> {
    pub cycles: Vec<Cycle<'a>>,
}

const CYCLE_EXPLANATION : &'static str = "\
Cycles in the build graph cause incorrect builds and are strictly forbidden.
Please edit the build description to remove the cycle(s) listed above.";

impl<'a> fmt::Display for CyclesError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f, "{} cycle(s) detected in the build graph...\n\n", self.cycles.len())?;

        for (i, cycle) in self.cycles.iter().enumerate() {
            write!(f, "Cycle {}\n", i+1)?;
            write!(f, "{}\n", cycle)?;
        }

        write!(f, "{}\n", CYCLE_EXPLANATION)?;

        Ok(())
    }
}

impl<'a> error::Error for CyclesError<'a> {
    fn description(&self) -> &str {
        "Cycle(s) detected in build graph."
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug)]
pub enum Error<'a> {
    Cycles(CyclesError<'a>),
}

impl<'a> From<CyclesError<'a>> for Error<'a> {
    fn from(err: CyclesError<'a>) -> Error<'a> {
        Error::Cycles(err)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Cycles(ref err) => write!(f, "{}", err),
        }
    }
}

impl<'a> error::Error for Error<'a> {
    fn description(&self) -> &str {
        match *self {
            Error::Cycles(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Cycles(ref err) => Some(err),
        }
    }
}

/// Creates a build graph from the given rules. This also checks for cycles and
/// race conditions. The graph is guaranteed to be bipartite.
pub fn from_rules<'a>(rules: &'a Rules) -> Result<BuildGraph<'a>, Error<'a>> {
    let mut graph = BuildGraph::new();

    for rule in rules.iter() {
        let task = graph.add_node(Node::Task(&rule.tasks));

        for r in &rule.inputs {
            let node = graph.add_node(Node::Resource(r));
            graph.add_edge(node, task, Edge::Explicit);
        }

        for r in &rule.outputs {
            let node = graph.add_node(Node::Resource(r));
            graph.add_edge(task, node, Edge::Explicit);
        }
    }

    // Check the graph to make sure the structure is sound.
    Ok(check_cycles(graph)?)
}

/// Checks for race conditions in the graph. That is, if any race has two or
/// more parents. In such a case where two tasks output the same resource,
/// depending on the order in which they get executed, they could be overwriting
/// each other's output.
fn check_races(graph: BuildGraph) -> Result<BuildGraph, Error> {
    Ok(graph)
}

/// Checks for cycles in the graph using Tarjan's algorithm for finding strongly
/// connected components.
fn check_cycles<'a>(graph: BuildGraph<'a>) -> Result<BuildGraph<'a>, CyclesError<'a>> {

    let mut cycles = Vec::new();

    for scc in tarjan_scc(&graph) {
        if scc.len() > 1 {
            // Only strongly connected components (SCCs) with more than 1 node
            // have a cycle.
            cycles.push(Cycle { nodes: scc });
        }
    }

    if cycles.is_empty() {
        Ok(graph)
    }
    else {
        Err(CyclesError { cycles: cycles })
    }
}
