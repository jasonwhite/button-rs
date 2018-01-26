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

//! In order for the build graph to be useful, it needs to do the following:
//!
//!  1. Perform a diff with another build graph in at most O(n) time. The diff
//!     must be over both the nodes and edges of the graph. This requires
//!     storing the graph's nodes and edges in sorted order (either internally
//!     or externally).

use std::error;
use std::fmt;
use std::cmp;

use std::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

use petgraph::visit::IntoNodeIdentifiers;
use petgraph::algo::tarjan_scc;
use petgraph::{graphmap, Direction};

use crossbeam;
use crossbeam::sync::MsQueue;

use res;
use task;
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
    Resource(&'a res::Any),
    Task(&'a task::List),
}

impl<'a> fmt::Display for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Resource(r) => write!(f, "res:{}", r),
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
    pub races: Vec<Race<&'a res::Any>>,
}

impl<'a> RaceError<'a> {
    pub fn new(mut races: Vec<Race<&'a res::Any>>) -> RaceError {
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

/// Checks for race conditions in the graph. That is, if any node has two or
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

    false
}

// Lock-free queue for nodes.
type Queue<T> = MsQueue<Option<T>>;

/// Traverses the graph in topological order. This function is the real meat of
/// the build system. Everything else is just support code.
///
/// The function `visit` is called for each node that is to be visited. If the
/// visitor function returns `true`, then its child nodes may be visited. It it
/// returns `false`, then its child nodes will not be visited. This is useful if
/// a resource is determined to be unchanged, obviating the need to do
/// additional work.
///
/// TODO: Use a priority queue. Tasks that take the longest to execute should be
/// started first to maximize efficiency. The first time the graph is traversed,
/// all nodes are considered equal (because it's impossible to accurately
/// predict the time it takes to execute a task without actually executing the
/// task). A predicate function can be provided to do the sorting.
pub fn traverse<F, E>(g: &BuildGraph,
                      visit: F,
                      threads: usize)
                      -> Result<(), Vec<E>>
    where F: Fn(usize, Node) -> Result<bool, E> + Send + Sync,
          E: Send
{
    // Always use at least one thread.
    let threads = cmp::max(threads, 1);

    // List of errors that occurred during the traversal.
    let errors = Mutex::new(Vec::new());

    // Nodes that have been visited. The value in this map indicates whether or
    // not the visitor function was called on it.
    let visited = Mutex::new(HashMap::new());

    // Start the traversal from all nodes that have no incoming edges.
    let roots = g.node_identifiers()
        .filter(move |&a| {
                    g.neighbors_directed(a, Direction::Incoming)
                        .next()
                        .is_none()
                });

    let queue = &Queue::new();

    let (cvar, mutex) = (&Condvar::new(), Mutex::new(()));

    // Keeps a count of the number of nodes being processed (or waiting to be
    // processed). When this reaches 0, we know there is no more work to do.
    let active = &AtomicUsize::new(0);

    crossbeam::scope(|scope| {
        let visited = &visited;
        let visit = &visit;
        let errors = &errors;

        for id in 0..threads {
            scope.spawn(move || {
                traversal_worker(id,
                                 queue,
                                 cvar,
                                 active,
                                 g,
                                 visited,
                                 visit,
                                 errors)
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

fn traversal_worker<'a, F, E>(id: usize,
                              queue: &Queue<Node<'a>>,
                              cvar: &Condvar,
                              active: &AtomicUsize,
                              g: &'a BuildGraph,
                              visited_arc: &Mutex<HashMap<Node<'a>, bool>>,
                              visit: &F,
                              errors: &Mutex<Vec<E>>)
    where F: Fn(usize, Node) -> Result<bool, E> + Sync,
          E: Send
{
    while let Some(node) = queue.pop() {
        // Only call the visitor function if:
        //  1. This node has no parents, or
        //  2. Any of its parents have had its visitor function called.
        //
        // Although the entire graph is traversed (unless an error occurs), we
        // may only call the visitor function on a subset of it.
        let do_visit = {
            let mut parents = g.neighbors_directed(node, Direction::Incoming);
            let visited = visited_arc.lock().unwrap();
            empty_or_any(&mut parents, |p| visited.get(&p) == Some(&true))
        };

        let keep_going = if do_visit { visit(id, node) } else { Ok(false) };

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

        for neigh in g.neighbors(node) {
            // Only visit a child node if that child's parents have all been
            // visited. There might be more efficient ways to do this. We could
            // keep a count of visited parents for each node instead.
            let mut parents = g.neighbors_directed(neigh, Direction::Incoming);
            if !visited.contains_key(&neigh) &&
               parents.all(|p| visited.contains_key(&p)) {
                active.fetch_add(1, Ordering::SeqCst);
                queue.push(Some(neigh));
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
    use task::Command;
    use std::path::PathBuf;

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

        assert!(from_rules(&rules).is_ok());
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
            "outputs": ["foobar", "foo.o"]
        }
        ]"#;

        let rules = Rules::from_str(&data).unwrap();

        let graph = from_rules(&rules);

        let foo = FilePath::from("foo.o").into();
        let bar = FilePath::from("bar.o").into();

        let races = vec![Race::new(&bar, 2), Race::new(&foo, 3)];

        assert_eq!(graph.unwrap_err(), Error::Races(RaceError::new(races)));
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

        let graph = from_rules(&rules);

        let foo_c = FilePath::from("foo.c").into();
        let task = vec![Command::new(PathBuf::from("gcc"),
                                     vec!["foo.c".to_owned()]).into()].into();

        let cycles = vec![Cycle::new(vec![Node::Resource(&foo_c),
                                          Node::Task(&task)])];

        assert_eq!(graph.unwrap_err(), Error::Cycles(CyclesError::new(cycles)));
    }
}
