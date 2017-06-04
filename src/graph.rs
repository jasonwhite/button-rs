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

use std::collections::HashMap;

use petgraph::visit::IntoNodeIdentifiers;
use petgraph::algo::tarjan_scc;
use petgraph::{graphmap, Direction};

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
    #[allow(dead_code)]
    Implicit,
}

/// A node in the graph.
#[derive(Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Node<'a> {
    Resource(&'a resources::FilePath),
    Task(&'a Vec<tasks::Command>),
}

impl<'a> fmt::Display for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Resource(r) => write!(f, "resource:{}", r),
            Node::Task(t) => write!(f, "task:{:?}", t),
        }
    }
}

/// The build graph.
pub type BuildGraph<'a> = graphmap::DiGraphMap<Node<'a>, Edge>;

/// A cycle in the graph. A cycle is denoted by the nodes contained in the
/// cycle. The nodes in the cycle should be in topological order. That is, each
/// node's parent must be the previous node in the list.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Cycle<'a> {
    nodes: Vec<Node<'a>>,
}

impl<'a> Cycle<'a> {
    pub fn new(nodes: Vec<Node<'a>>) -> Cycle {
        Cycle { nodes: nodes }
    }
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
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CyclesError<'a> {
    pub cycles: Vec<Cycle<'a>>,
}

impl<'a> CyclesError<'a> {
    pub fn new(mut cycles: Vec<Cycle<'a>>) -> CyclesError {
        // Sort to avoid non-determinism in the output and to make testing
        // easier.
        cycles.sort();

        CyclesError { cycles: cycles }
    }
}

const CYCLE_EXPLANATION: &'static str = "\
Cycles in the build graph cause incorrect builds and are strictly forbidden.
Please edit the build description to remove the cycle(s) listed above.";

impl<'a> fmt::Display for CyclesError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f,
               "{} cycle(s) detected in the build graph...\n\n",
               self.cycles.len())?;

        for (i, cycle) in self.cycles.iter().enumerate() {
            write!(f, "Cycle {}\n", i + 1)?;
            write!(f, "{}\n", cycle)?;
        }

        write!(f, "{}", CYCLE_EXPLANATION)
    }
}

impl<'a> error::Error for CyclesError<'a> {
    fn description(&self) -> &str {
        "Cycle(s) detected in the build graph."
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// A race condition in the build graph.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Race<N> {
    /// The node with two or more incoming edges.
    pub node: N,

    /// The number of incoming edges.
    pub count: usize,
}

impl<N> Race<N> {
    fn new(node: N, count: usize) -> Race<N> {
        Race {
            node: node,
            count: count,
        }
    }
}

impl<N> fmt::Display for Race<N>
    where N: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} (output of {} tasks)", self.node, self.count)
    }
}

/// Error when one or more race conditions are detected in the build graph.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RaceError<'a> {
    pub races: Vec<Race<&'a resources::FilePath>>,
}

impl<'a> RaceError<'a> {
    pub fn new(mut races: Vec<Race<&'a resources::FilePath>>) -> RaceError {
        // Sort to avoid non-determinism in the output and to make testing
        // easier.
        races.sort();

        RaceError { races: races }
    }
}

const RACE_EXPLANATION: &'static str = "\
Race conditions in the build graph cause incorrect incremental builds and are
strictly forbidden. The resources listed above are the output of more than one
task. Depending on the order in which the task is executed, one task will
overwrite the output of the other. Please edit the build description to fix the
race condition(s).";

impl<'a> fmt::Display for RaceError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f,
               "{} race condition(s) detected in the build graph:\n\n",
               self.races.len())?;

        for race in &self.races {
            write!(f, " - {}\n", race)?;
        }

        write!(f, "\n{}", RACE_EXPLANATION)
    }
}

impl<'a> error::Error for RaceError<'a> {
    fn description(&self) -> &str {
        "Race condition(s) detected in the build graph."
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Error<'a> {
    Races(RaceError<'a>),
    Cycles(CyclesError<'a>),
}

impl<'a> From<RaceError<'a>> for Error<'a> {
    fn from(err: RaceError<'a>) -> Error<'a> {
        Error::Races(err)
    }
}

impl<'a> From<CyclesError<'a>> for Error<'a> {
    fn from(err: CyclesError<'a>) -> Error<'a> {
        Error::Cycles(err)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Races(ref err) => write!(f, "{}", err),
            Error::Cycles(ref err) => write!(f, "{}", err),
        }
    }
}

impl<'a> error::Error for Error<'a> {
    fn description(&self) -> &str {
        match *self {
            Error::Races(ref err) => err.description(),
            Error::Cycles(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Races(ref err) => Some(err),
            Error::Cycles(ref err) => Some(err),
        }
    }
}

/// Creates a build graph from the given rules. If the graph would contain race
/// conditions or cycles, an error is returned. Thus, the returned graph is
/// guaranteed to be bipartite and acyclic.
pub fn from_rules(rules: &Rules) -> Result<BuildGraph, Error> {
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
    Ok(check_cycles(check_races(graph)?)?)
}

/// Checks for race conditions in the graph. That is, if any race has two or
/// more parents. In such a case where two tasks output the same resource,
/// depending on the order in which they get executed, they could be overwriting
/// each other's output.
fn check_races(graph: BuildGraph) -> Result<BuildGraph, RaceError> {
    let mut races = Vec::new();

    for node in graph.nodes() {
        match node {
            Node::Resource(r) => {
                let incoming =
                    graph.neighbors_directed(node, Direction::Incoming).count();

                if incoming > 1 {
                    races.push(Race::new(r, incoming));
                }
            }
            Node::Task(_) => {}
        };
    }

    if races.is_empty() {
        Ok(graph)
    } else {
        Err(RaceError::new(races))
    }
}

/// Checks for cycles in the graph using Tarjan's algorithm for finding strongly
/// connected components.
fn check_cycles(graph: BuildGraph) -> Result<BuildGraph, CyclesError> {

    let mut cycles = Vec::new();

    for scc in tarjan_scc(&graph) {
        if scc.len() > 1 {
            // Only strongly connected components (SCCs) with more than 1 node
            // have a cycle.
            cycles.push(Cycle::new(scc));
        }
    }

    if cycles.is_empty() {
        Ok(graph)
    } else {
        Err(CyclesError::new(cycles))
    }
}

// This should be in the standard library.
fn empty_or_any<I, F>(iter: &mut I, mut f: F) -> bool
    where I: Iterator,
          F: FnMut(I::Item) -> bool
{
    match iter.next() {
        None => return true, // Empty
        Some(x) => {
            if f(x) {
                return true;
            }
        }
    };

    for x in iter {
        if f(x) {
            return true;
        }
    }

    return false;
}

/// Traverses the graph in topological order. This function is the real meat of
/// the build system. Everything else is just support code.
///
/// The function `visit` is called for each node that is to be visited. If the
/// visitor function returns `true`, then its child nodes may be visited. It it
/// returns `false`, then its child nodes will not be visited. This is useful if
/// a resource is determined to be unchanged, obviating the need to do
/// additional work.
///
/// TODO: Return `Result<(), ErrorList>`.
///
/// TODO: Do the traversal in parallel.
///
/// TODO: Instead of a stack, keep a heap (i.e., priority queue) of nodes to
/// visit next. Elements at the top of the heap would be visited first. The heap
/// would be sorted according to the estimated cost of visiting a node. For
/// example, tasks that take the longest to execute should be started first to
/// maximize efficiency.  The first time the graph is traversed, all nodes are
/// considered equal (because it's impossible accurately to predict the time it
/// takes to execute a task without actually executing the task). A predicate
/// function can be provided to do the sorting.
pub fn traverse<F>(g: &BuildGraph, visit: F)
    where F: Fn(Node) -> bool
{
    // Nodes that need to be visited.
    let mut tovisit = Vec::new();

    // Nodes that have been visited. The value in this map indicates whether or
    // not the visitor function was called on it.
    let mut visited = HashMap::new();

    // Start the traversal from all nodes that have no incoming edges.
    tovisit.extend(g.node_identifiers()
                       .filter(move |&a| {
                                   g.neighbors_directed(a, Direction::Incoming)
                                       .next()
                                       .is_none()
                               }));

    while let Some(node) = tovisit.pop() {
        if visited.contains_key(&node) {
            continue;
        }

        // Only call the visitor function if:
        //  1. This node has no parents, or
        //  2. Any of its parents have had its visitor function called.
        //
        // The whole graph must be traversed (unless an error occurs), but we
        // only want to call the visitor function on a subset of it.
        let mut parents = g.neighbors_directed(node, Direction::Incoming);
        if empty_or_any(&mut parents, |p| visited.get(&p) == Some(&true)) {
            visited.insert(node, visit(node));
        } else {
            visited.insert(node, false);
        }

        for neigh in g.neighbors(node) {
            // Only visit a child node if that child's parents have all been
            // visited. There are more efficient ways to do this. We could keep
            // a count of visited parents for each node instead.
            let mut parents = g.neighbors_directed(neigh, Direction::Incoming);
            if parents.all(|p| visited.contains_key(&p)) {
                tovisit.push(neigh);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resources::FilePath;
    use tasks::Command;

    #[test]
    fn test_good_graph() {
        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "-c", "foo.c", "-o", "foo.o"]}
            ],
            "outputs": ["foo.o"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "-c", "bar.c", "-o", "bar.o"]}
            ],
            "outputs": ["bar.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {"args": ["gcc", "foo.o", "bar.o", "-o", "foobar"]}
            ],
            "outputs": ["foobar"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        assert!(from_rules(&rules).is_ok());
    }

    #[test]
    fn test_races() {

        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "-c", "foo.c", "-o", "foo.o"]}
            ],
            "outputs": ["foo.o", "bar.o"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "-c", "bar.c", "-o", "bar.o"]}
            ],
            "outputs": ["bar.o", "foo.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {"args": ["gcc", "foo.o", "bar.o", "-o", "foobar"]}
            ],
            "outputs": ["foobar", "foo.o"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = from_rules(&rules);

        let foo = FilePath::from("foo.o");
        let bar = FilePath::from("bar.o");

        let races = vec![Race::new(&bar, 2), Race::new(&foo, 3)];

        assert_eq!(graph.unwrap_err(), Error::Races(RaceError::new(races)));
    }

    #[test]
    fn test_cycles() {

        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "foo.c"]}
            ],
            "outputs": ["foo.o", "foo.c"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {"args": ["gcc", "bar.c"]}
            ],
            "outputs": ["bar.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {"args": ["gcc", "foo.o", "bar.o", "-o", "foobar"]}
            ],
            "outputs": ["foobar"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = from_rules(&rules);

        let foo_c = FilePath::from("foo.c");
        let task = vec![Command::new(vec!["gcc".to_owned(),
                                          "foo.c".to_owned()],
                                     None,
                                     None)];

        let cycles = vec![Cycle::new(vec![Node::Resource(&foo_c),
                                          Node::Task(&task)])];

        assert_eq!(graph.unwrap_err(), Error::Cycles(CyclesError::new(cycles)));
    }
}
