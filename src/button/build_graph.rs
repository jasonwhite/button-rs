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

use std::cmp;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};

use crossbeam;
use crossbeam::sync::MsQueue;

use graph::{tarjan_scc, Graph, NodeTrait};

use res;
use task;

use rules::{Rule, Rules};
use util::empty_or_any;

/// A node in the graph.
#[derive(Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Node {
    Resource(res::Any),
    Task(task::List),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Node::Resource(ref x) => write!(f, "({})", x),
            &Node::Task(ref x) => write!(f, "[{}]", x),
        }
    }
}

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

/// Error for when one or more cycles are detected in the build graph.
#[derive(Eq, PartialEq)]
pub struct CyclesError<N, E>
where
    N: NodeTrait,
{
    pub graph: Graph<N, E>,
    pub cycles: Vec<Vec<usize>>,
}

impl<N, E> CyclesError<N, E>
where
    N: NodeTrait,
{
    pub fn new(
        graph: Graph<N, E>,
        cycles: Vec<Vec<usize>>,
    ) -> CyclesError<N, E> {
        CyclesError {
            graph: graph,
            cycles: cycles,
        }
    }
}

const CYCLE_EXPLANATION: &'static str = "\
Cycles in the build graph cause incorrect builds and are strictly forbidden.
Please edit the build description to remove the cycle(s) listed above.";

impl<N, E> fmt::Display for CyclesError<N, E>
where
    N: NodeTrait + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
                .map(|index| self.graph.node(*index).unwrap());

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
#[derive(Fail, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
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

        RaceError { races: races }
    }
}

const RACE_EXPLANATION: &'static str = "\
Race conditions in the build graph cause incorrect incremental builds and are
strictly forbidden. The resources listed above are the output of more than one
task. Depending on the order in which the task is executed, one task will
overwrite the output of the other. Please edit the build description to fix the
race condition(s).";

impl fmt::Display for RaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} race condition(s) detected in the build graph:\n\n",
            self.races.len()
        )?;

        for race in &self.races {
            write!(f, " - {}\n", race)?;
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
    Cycles(CyclesError<Node, Edge>),
}

impl From<RaceError> for Error {
    fn from(err: RaceError) -> Error {
        Error::Races(err)
    }
}

impl From<CyclesError<Node, Edge>> for Error {
    fn from(err: CyclesError<Node, Edge>) -> Error {
        Error::Cycles(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Races(ref err) => write!(f, "{}", err),
            Error::Cycles(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for Error {
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

// Lock-free queue for nodes.
type Queue<T> = MsQueue<Option<T>>;

/// The build graph.
///
/// This is the core data structure of the build system. It is a bi-partite
/// acyclic graph. We guarantee that the graph is free of cycles and race
/// conditions when constructing it from a set of rules.
pub struct BuildGraph {
    graph: Graph<Node, Edge>,
}

impl BuildGraph {
    /// Creates a build graph from the given rules. If the graph would contain
    /// race conditions or cycles, an error is returned. Thus, the returned
    /// graph is guaranteed to be bipartite and acyclic.
    pub fn from_rules(rules: Rules) -> Result<BuildGraph, Error> {
        let mut g = Graph::new();

        for rule in rules.into_iter() {
            let Rule {
                inputs,
                outputs,
                tasks,
            } = rule;

            let task = g.add_node(Node::Task(tasks));

            for r in inputs {
                let node = g.add_node(Node::Resource(r));
                g.add_edge(node, task, Edge::Explicit);
            }

            for r in outputs {
                let node = g.add_node(Node::Resource(r));
                g.add_edge(task, node, Edge::Explicit);
            }
        }

        Ok(BuildGraph {
            graph: check_races(check_cycles(g)?)?,
        })
    }

    /// Traverses the graph in topological order. This function is the real
    /// meat of the build system. Everything else is just support code.
    ///
    /// The function `visit` is called for each node that is to be visited. If
    /// the visitor function returns `true`, then its child nodes may be
    /// visited. If it returns `false`, then its child nodes will not be
    /// visited. This is useful if a resource is determined to be
    /// unchanged, obviating the need to do additional work.
    ///
    /// TODO: Use a priority queue. Tasks that take the longest to execute
    /// should be started first to maximize efficiency. The first time the
    /// graph is traversed, all nodes are considered equal (because it's
    /// impossible to accurately predict the time it takes to execute a
    /// task without actually executing the task). A predicate function can
    /// be provided to do the sorting.
    pub fn traverse<F, E>(&self, visit: F, threads: usize) -> Result<(), Vec<E>>
    where
        F: Fn(usize, &Node) -> Result<bool, E> + Send + Sync,
        E: Send,
    {
        // Always use at least one thread.
        let threads = cmp::max(threads, 1);

        // List of errors that occurred during the traversal.
        let errors = Mutex::new(Vec::new());

        // Nodes that have been visited. The value in this map indicates
        // whether or not the visitor function was called on it.
        let visited = Mutex::new(HashMap::new());

        // Start the traversal from all nodes that have no incoming edges.
        let roots = self.graph.roots();

        let queue = &Queue::new();

        let (cvar, mutex) = (&Condvar::new(), Mutex::new(()));

        // Keeps a count of the number of nodes being processed (or waiting to
        // be processed). When this reaches 0, we know there is no more
        // work to do.
        let active = &AtomicUsize::new(0);

        crossbeam::scope(|scope| {
            let visited = &visited;
            let visit = &visit;
            let errors = &errors;

            for id in 0..threads {
                scope.spawn(move || {
                    traversal_worker(
                        id,
                        queue,
                        cvar,
                        active,
                        &self.graph,
                        visited,
                        visit,
                        errors,
                    )
                });
            }

            // Queue the root nodes.
            for node in roots {
                active.fetch_add(1, Ordering::SeqCst);
                queue.push(Some(node));
            }

            let mut guard = mutex.lock().unwrap();
            while active.load(Ordering::SeqCst) > 0 {
                // Wait until all queued items are complete.
                guard = cvar.wait(guard).unwrap();
            }

            // Send message to shutdown all threads.
            for _ in 0..threads {
                queue.push(None);
            }
        });

        let errors = errors.into_inner().unwrap();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Checks for race conditions in the graph. That is, if any node has two or
/// more parents. In such a case where two tasks output the same resource,
/// depending on the order in which they get executed, they could be overwriting
/// each other's output.
fn check_races(
    graph: Graph<Node, Edge>,
) -> Result<Graph<Node, Edge>, RaceError> {
    let mut races = Vec::new();

    for (i, node) in graph.nodes().enumerate() {
        match node {
            &Node::Resource(ref r) => {
                let incoming = graph.incoming(i).count();

                if incoming > 1 {
                    races.push(Race::new(r.clone(), incoming));
                }
            }
            &Node::Task(_) => {}
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

    for scc in tarjan_scc(&graph) {
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

fn traversal_worker<'a, F, E>(
    id: usize,
    queue: &Queue<usize>,
    cvar: &Condvar,
    active: &AtomicUsize,
    g: &'a Graph<Node, Edge>,
    visited_arc: &Mutex<HashMap<usize, bool>>,
    visit: &F,
    errors: &Mutex<Vec<E>>,
) where
    F: Fn(usize, &'a Node) -> Result<bool, E> + Sync,
    E: Send,
{
    while let Some(node) = queue.pop() {
        // Only call the visitor function if:
        //  1. This node has no incoming edges, or
        //  2. Any of its incoming nodes have had its visitor function called.
        //
        // Although the entire graph is traversed (unless an error occurs), we
        // may only call the visitor function on a subset of it.
        let do_visit = {
            let mut incoming = g.incoming(node);
            let visited = visited_arc.lock().unwrap();
            empty_or_any(&mut incoming, |p| visited.get(&p) == Some(&true))
        };

        let keep_going = if do_visit {
            visit(id, g.node(node).unwrap())
        } else {
            Ok(false)
        };

        let mut visited = visited_arc.lock().unwrap();

        match keep_going {
            Ok(keep_going) => visited.insert(node, keep_going),
            Err(err) => {
                let mut errors = errors.lock().unwrap();
                errors.push(err);
                visited.insert(node, false);

                // In case of error, do not traverse child nodes. Nothing
                // that depends on this node should be visited.
                active.fetch_sub(1, Ordering::SeqCst);
                cvar.notify_one();
                continue;
            }
        };

        for neigh in g.outgoing(node) {
            // Only visit a node if that node's incoming nodes have all been
            // visited. There might be more efficient ways to do this.
            let mut incoming = g.incoming(*neigh);
            if !visited.contains_key(neigh)
                && incoming.all(|p| visited.contains_key(&p))
            {
                active.fetch_add(1, Ordering::SeqCst);
                queue.push(Some(*neigh));
            }
        }

        // Notify that an element was taken off of the queue. When the queue
        // becomes empty, a message is sent to each worker thread to shutdown.
        active.fetch_sub(1, Ordering::SeqCst);
        cvar.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use res::FilePath;

    #[test]
    fn test_good_graph() {
        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["-c", "foo.c", "-o", "foo.o"]
                }
            ],
            "outputs": ["foo.o"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["-c", "bar.c", "-o", "bar.o"]
                }
            ],
            "outputs": ["bar.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["foo.o", "bar.o", "-o", "foobar"]
                }
            ],
            "outputs": ["foobar"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        assert!(BuildGraph::from_rules(rules).is_ok());
    }

    #[test]
    fn test_races() {
        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["-c", "foo.c", "-o", "foo.o"]
                }
            ],
            "outputs": ["foo.o", "bar.o"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["-c", "bar.c", "-o", "bar.o"]
                }
            ],
            "outputs": ["bar.o", "foo.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["foo.o", "bar.o", "-o", "foobar"]
                }
            ],
            "outputs": ["foobar"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = BuildGraph::from_rules(rules);

        let foo = FilePath::from("foo.o").into();
        let bar = FilePath::from("bar.o").into();

        let races = vec![Race::new(bar, 2), Race::new(foo, 2)];

        assert_eq!(graph.err(), Some(Error::Races(RaceError::new(races))));
    }

    #[test]
    fn test_cycles() {
        let data = r#"[
        {
            "inputs": ["foo.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["foo.c"]
                }
            ],
            "outputs": ["foo.o", "foo.c"]
        },
        {
            "inputs": ["bar.c", "foo.h"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["bar.c"]
                }
            ],
            "outputs": ["bar.o"]
        },
        {
            "inputs": ["foo.o", "bar.o"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["foo.o", "bar.o", "-o", "foobar"]
                }
            ],
            "outputs": ["foobar"]
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
