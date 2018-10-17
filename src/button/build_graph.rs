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
use std::io;

use graph::{
    Algo, Edges, Graph, Graphviz, Neighbors, NodeIndexable, NodeTrait, Nodes, NodeIndex,
};

use res;
use task;

use rules::{Rule, Rules};

/// A node in the graph.
#[derive(
    Serialize, Deserialize, Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Debug,
)]
pub enum Node {
    Resource(res::Any),
    Task(task::List),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Resource(ref x) => write!(f, "({})", x),
            Node::Task(ref x) => write!(f, "[{}]", x),
        }
    }
}

/// Error for when one or more cycles are detected in the build graph.
#[derive(Eq, PartialEq)]
pub struct CyclesError<N, E>
where
    N: NodeTrait,
{
    pub graph: Graph<N, E>,
    pub cycles: Vec<Vec<NodeIndex>>,
}

impl<N, E> CyclesError<N, E>
where
    N: NodeTrait,
{
    pub fn new(
        graph: Graph<N, E>,
        cycles: Vec<Vec<NodeIndex>>,
    ) -> CyclesError<N, E> {
        CyclesError { graph, cycles }
    }
}

const CYCLE_EXPLANATION: &str = "\
Cycles in the build graph cause incorrect builds and are strictly forbidden.
Please edit the build description to remove the cycle(s) listed above.";

impl<N, E> fmt::Display for CyclesError<N, E>
where
    N: NodeTrait + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use graph::NodeIndexable;

        writeln!(
            f,
            "{} cycle(s) detected in the build graph...\n",
            self.cycles.len()
        )?;

        for (i, cycle) in self.cycles.iter().enumerate() {
            writeln!(f, "Cycle {}", i + 1)?;

            // The nodes in a cycle are listed in reverse topological order.
            // Thus, we need to print them out in reverse order so that it makes
            // more sense.
            let mut it = cycle
                .iter()
                .rev()
                .map(|index| self.graph.from_index(*index));

            // Unwrapping because there must always be at least one node in a
            // cycle. If this panics, then the code creating the
            // cycle is buggy.
            let first = it.next().unwrap();

            writeln!(f, "    {}", first)?;

            for node in it {
                writeln!(f, " -> {}", node)?;
            }

            // Make the cycle obvious
            writeln!(f, " -> {}", first)?;
        }

        write!(f, "\n{}", CYCLE_EXPLANATION)
    }
}

impl<N, E> fmt::Debug for CyclesError<N, E>
where
    N: NodeTrait + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<N, E> error::Error for CyclesError<N, E>
where
    N: NodeTrait + fmt::Display,
{
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
        Race { node, count }
    }
}

impl<N> fmt::Display for Race<N>
where
    N: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} (output of {} tasks)", self.node, self.count)
    }
}

/// Error when one or more race conditions are detected in the build graph.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RaceError {
    pub races: Vec<Race<res::Any>>,
}

impl RaceError {
    pub fn new(mut races: Vec<Race<res::Any>>) -> RaceError {
        // Sort to avoid non-determinism in the output and to make testing
        // easier.
        races.sort();

        RaceError { races }
    }
}

const RACE_EXPLANATION: &str = "\
Race conditions in the build graph cause incorrect incremental builds and are
strictly forbidden. The resources listed above are the output of more than one
task. Depending on the order in which the task is executed, one task will
overwrite the output of the other. Please edit the build description to fix the
race condition(s).";

impl fmt::Display for RaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{} race condition(s) detected in the build graph:\n",
            self.races.len()
        )?;

        for race in &self.races {
            writeln!(f, " - {}", race)?;
        }

        write!(f, "\n{}", RACE_EXPLANATION)
    }
}

impl error::Error for RaceError {
    fn description(&self) -> &str {
        "Race condition(s) detected in the build graph."
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Races(RaceError),
    Cycles(CyclesError<Node, ()>),
}

impl From<RaceError> for Error {
    fn from(err: RaceError) -> Error {
        Error::Races(err)
    }
}

impl From<CyclesError<Node, ()>> for Error {
    fn from(err: CyclesError<Node, ()>) -> Error {
        Error::Cycles(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Races(ref err) => write!(f, "{}", err),
            Error::Cycles(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Races(ref err) => err.description(),
            Error::Cycles(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::Races(ref err) => Some(err),
            Error::Cycles(ref err) => Some(err),
        }
    }
}

pub type BuildGraph = Graph<Node, ()>;

pub type SuperBuildGraph = Graph<Graph<Node, ()>, ()>;

/// The build graph.
///
/// This is the core data structure of the build system. It is a bi-partite
/// acyclic graph. We guarantee that the graph is free of cycles and race
/// conditions when constructing it from a set of rules.
pub trait FromRules: Sized {
    fn from_rules(rules: Rules) -> Result<Self, Error>;
}

impl FromRules for BuildGraph {
    /// Creates a build graph from the given rules. If the graph would contain
    /// race conditions or cycles, an error is returned. Thus, the returned
    /// graph is guaranteed to be bipartite and acyclic.
    fn from_rules(rules: Rules) -> Result<BuildGraph, Error> {
        let mut g = Graph::new();

        for rule in rules {
            let Rule {
                inputs,
                outputs,
                tasks,
            } = rule;

            let task = g.add_node(Node::Task(tasks));

            for r in inputs {
                let node = g.add_node(Node::Resource(r));
                g.add_edge(node, task, ());
            }

            for r in outputs {
                let node = g.add_node(Node::Resource(r));
                g.add_edge(task, node, ());
            }
        }

        Ok(check_races(check_cycles(g)?)?)
    }
}

impl Graphviz for BuildGraph {
    fn graphviz(&self, f: &mut io::Write) -> Result<(), io::Error> {
        fn escape_label(s: &str) -> String {
            s.chars().flat_map(|c| c.escape_default()).collect()
        }

        writeln!(f, "digraph G {{")?;

        // Style and label every resource
        writeln!(f, "    subgraph {{")?;
        writeln!(
            f,
            "        node [ \
             shape=ellipse, \
             fillcolor=lightskyblue2, \
             style=filled];"
        )?;
        for i in self.nodes() {
            let node = self.from_index(i);
            match node {
                Node::Resource(ref resource) => {
                    writeln!(f, "        N{} [label={}];", i, resource)?;
                }
                Node::Task(_) => {}
            };
        }
        writeln!(f, "    }}")?;

        // Style and label every task
        writeln!(f, "    subgraph {{")?;
        writeln!(
            f,
            "        node [shape=box, fillcolor=gray91, style=filled];"
        )?;
        for i in self.nodes() {
            let node = self.from_index(i);
            match node {
                Node::Resource(_) => {}
                Node::Task(ref task) => {
                    writeln!(
                        f,
                        "        N{} [label=\"{}\"];",
                        i,
                        escape_label(&format!("{}", task))
                    )?;
                }
            };
        }
        writeln!(f, "    }}")?;

        // Edges
        for (from, to, _weight) in self.edges() {
            writeln!(f, "    N{} -> N{};", from, to)?;
        }

        writeln!(f, "}}")
    }
}

/// Checks for race conditions in the graph. That is, if any node has two or
/// more parents. In such a case where two tasks output the same resource,
/// depending on the order in which they get executed, they could be overwriting
/// each other's output.
fn check_races(graph: Graph<Node, ()>) -> Result<Graph<Node, ()>, RaceError> {
    let mut races = Vec::new();

    for i in graph.nodes() {
        let node = graph.from_index(i);
        match node {
            Node::Resource(ref r) => {
                let incoming = graph.incoming(i).count();

                if incoming > 1 {
                    races.push(Race::new(r.clone(), incoming));
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
fn check_cycles<N, E>(
    graph: Graph<N, E>,
) -> Result<Graph<N, E>, CyclesError<N, E>>
where
    N: NodeTrait,
{
    let mut cycles = Vec::new();

    for scc in graph.tarjan_scc() {
        if scc.len() > 1 {
            // Only strongly connected components (SCCs) with more than 1 node
            // have a cycle.
            cycles.push(scc);
        }
    }

    if cycles.is_empty() {
        Ok(graph)
    } else {
        Err(CyclesError::new(graph, cycles))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use res::File;

    #[test]
    fn test_good_graph() {
        let data = r#"[
        {
            "inputs": [{"file": "foo.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["-c", "foo.c", "-o", "foo.o"]
                    }
                }
            ],
            "outputs": [{"file": "foo.o"}]
        },
        {
            "inputs": [{"file": "bar.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["-c", "bar.c", "-o", "bar.o"]
                    }
                }
            ],
            "outputs": [{"file": "bar.o"}]
        },
        {
            "inputs": [{"file": "foo.o"}, {"file": "bar.o"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["foo.o", "bar.o", "-o", "foobar"]
                    }
                }
            ],
            "outputs": [{"file": "foobar"}]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        assert!(BuildGraph::from_rules(rules).is_ok());
    }

    #[test]
    fn test_races() {
        let data = r#"[
        {
            "inputs": [{"file": "foo.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["-c", "foo.c", "-o", "foo.o"]
                    }
                }
            ],
            "outputs": [{"file": "foo.o"}, {"file": "bar.o"}]
        },
        {
            "inputs": [{"file": "bar.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["-c", "bar.c", "-o", "bar.o"]
                    }
                }
            ],
            "outputs": [{"file": "bar.o"}, {"file": "foo.o"}]
        },
        {
            "inputs": [{"file": "foo.o"}, {"file": "bar.o"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["foo.o", "bar.o", "-o", "foobar"]
                    }
                }
            ],
            "outputs": [{"file": "foobar"}]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = BuildGraph::from_rules(rules);

        let foo = File::from("foo.o").into();
        let bar = File::from("bar.o").into();

        let races = vec![Race::new(bar, 2), Race::new(foo, 2)];

        assert_eq!(graph.err(), Some(Error::Races(RaceError::new(races))));
    }

    #[test]
    fn test_cycles() {
        let data = r#"[
        {
            "inputs": [{"file": "foo.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["foo.c"]
                    }
                }
            ],
            "outputs": [{"file": "foo.o"}, {"file": "foo.c"}]
        },
        {
            "inputs": [{"file": "bar.c"}, {"file": "foo.h"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["bar.c"]
                    }
                }
            ],
            "outputs": [{"file": "bar.o"}]
        },
        {
            "inputs": [{"file": "foo.o"}, {"file": "bar.o"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["foo.o", "bar.o", "-o", "foobar"]
                    }
                }
            ],
            "outputs": [{"file": "foobar"}]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = BuildGraph::from_rules(rules);

        assert_eq!(
            match graph.err().unwrap() {
                Error::Cycles(CyclesError { graph: _, cycles }) => cycles,
                _ => panic!(),
            }.len(),
            1
        );
    }
}
