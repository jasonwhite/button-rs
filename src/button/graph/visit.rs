// Copyright (c) 2018 Jason White
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
use std::hash::{BuildHasher, Hash};
use std::io;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crossbeam;

use util;

// Use a random queue. On average, this seems to have better CPU utilization.
type Queue<T> = util::RandomQueue<Option<T>>;

// Much of these traits were lifted from the petgraph crate. These are needed in
// order to have generic algorithms over the graph (e.g., DFS, BFS, topological
// traversal).

pub trait GraphBase {
    /// The node identifier.
    type Node;

    /// Edge data.
    type Edge;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;
}

/// A graph that is indexable.
pub trait NodeIndexable<'a>: GraphBase
where
    Self::Node: 'a,
{
    /// Convert an index to a node.
    ///
    /// Panics if the index is out of bounds.
    fn from_index(&'a self, index: usize) -> &'a Self::Node;

    /// Convert a node to an index if it exists.
    ///
    /// Returns `None` if the node does not exist in the graph.
    fn to_index(&self, node: &Self::Node) -> Option<usize>;

    /// Returns `true` if the node exists in the graph.
    fn contains_node(&self, n: &Self::Node) -> bool {
        self.to_index(n).is_some()
    }
}

/// Trait for iterating over the neighbors of nodes.
pub trait Neighbors<'a>: GraphBase {
    type Neighbors: Iterator<Item = usize>;

    /// Returns an iterator over all the incoming edges for the given node.
    ///
    /// Panics if the index does not exist.
    fn incoming(&'a self, node: usize) -> Self::Neighbors;

    /// Returns an iterator over all the outgoing edges for the given node.
    ///
    /// Panics if the index does not exist.
    fn outgoing(&'a self, node: usize) -> Self::Neighbors;

    fn neighbors(&'a self, node: usize, reverse: bool) -> Self::Neighbors {
        if reverse {
            self.incoming(node)
        } else {
            self.outgoing(node)
        }
    }

    /// Returns true if the given node is a root node (i.e., it has no incoming
    /// edges).
    fn is_root_node(&'a self, node: usize) -> bool {
        self.incoming(node).next().is_none()
    }

    /// Returns true if the given node is a terminal node (i.e., it has no
    /// outgoing edges).
    fn is_terminal_node(&'a self, node: usize) -> bool {
        self.outgoing(node).next().is_none()
    }
}

/// Trait for iterating over the nodes in the graph.
pub trait Nodes<'a>: GraphBase
where
    Self::Node: 'a,
{
    type Iter: Iterator<Item = usize>;

    /// Returns an iterator over the nodes in the graph.
    fn nodes(&'a self) -> Self::Iter;
}

/// Trait for iterating over the edges in the graph.
pub trait Edges<'a>: GraphBase
where
    Self::Edge: 'a,
{
    type Iter: Iterator<Item = (usize, usize, &'a Self::Edge)>;

    /// Returns an iterator over the edges in the graph.
    fn edges(&'a self) -> Self::Iter;
}

/// A mapping for storing visited status.
pub trait VisitMap<N, T> {
    /// Marks the node as visited. Returns the previous visited state if any.
    fn visit(&mut self, node: N, value: T) -> Option<T>;

    /// Returns `Some(&T)` if this node has been visited. Returns `None` if it
    /// hasn't been visited.
    fn is_visited(&self, node: &N) -> Option<&T>;

    /// Marks all nodes as unvisited.
    fn clear(&mut self);
}

impl<N, T, S> VisitMap<N, T> for HashMap<N, T, S>
where
    N: Eq + Hash,
    S: BuildHasher,
{
    fn visit(&mut self, node: N, value: T) -> Option<T> {
        self.insert(node, value)
    }

    fn is_visited(&self, node: &N) -> Option<&T> {
        self.get(node)
    }

    fn clear(&mut self) {
        HashMap::clear(self)
    }
}

impl<T> VisitMap<usize, T> for Vec<Option<T>> {
    fn visit(&mut self, node: usize, value: T) -> Option<T> {
        mem::replace(&mut self[node], Some(value))
    }

    fn is_visited(&self, node: &usize) -> Option<&T> {
        match &self[*node] {
            Some(x) => Some(&x),
            None => None,
        }
    }

    fn clear(&mut self) {
        let len = self.len();

        Vec::clear(self);

        for _ in 0..len {
            self.push(None);
        }
    }
}

/// A graph that can track the visited status of its nodes.
pub trait Visitable<T> {
    type Map: VisitMap<usize, T>;

    /// Creates a new visit map.
    fn visit_map(&self) -> Self::Map;
}

/// The state passed to worker threads when traversing the graph.
struct TraversalState<G, E>
where
    G: Visitable<bool>,
    E: Send,
{
    // List of errors that occurred during the traversal.
    pub errors: Mutex<Vec<(usize, E)>>,

    // Nodes that have been visited. The value in this map indicates
    // whether or not the visitor function was called on it.
    pub visited: Mutex<G::Map>,

    // Queue of node indices. All the items in the queue have the property of
    // not depending on each other.
    pub queue: Queue<usize>,

    // Keeps a count of the number of nodes being processed (or waiting to
    // be processed). When this reaches 0, we know there is no more
    // work to do.
    pub active: AtomicUsize,
}

impl<'a, G, E> TraversalState<G, E>
where
    G: Visitable<bool> + Algo<'a>,
    E: Send,
{
    pub fn new(graph: &'a G, reverse: bool) -> TraversalState<G, E> {
        let queue = Queue::new();

        let active = if reverse {
            queue.push_many(graph.terminal_nodes().map(|x| Some(x)))
        } else {
            queue.push_many(graph.root_nodes().map(|x| Some(x)))
        };

        TraversalState {
            errors: Mutex::new(Vec::new()),
            visited: Mutex::new(graph.visit_map()),
            queue,
            active: AtomicUsize::new(active),
        }
    }
}

pub trait Algo<'a>: Nodes<'a> + Neighbors<'a>
where
    Self: Sized + 'a,
{
    /// Returns an iterator over all nodes that have no incoming edges.
    fn root_nodes(&'a self) -> RootNodes<'a, Self> {
        RootNodes::new(self)
    }

    /// Returns an iterator over all nodes that have at least one incoming
    /// edge.
    fn non_root_nodes(&'a self) -> NonRootNodes<'a, Self> {
        NonRootNodes::new(self)
    }

    /// Returns an iterator over all nodes that have no outgoing edges.
    fn terminal_nodes(&'a self) -> TerminalNodes<'a, Self> {
        TerminalNodes::new(self)
    }

    /// Returns an iterator over all nodes that have at least one outgoing edge.
    fn non_terminal_nodes(&'a self) -> NonTerminalNodes<'a, Self> {
        NonTerminalNodes::new(self)
    }

    /// Returns the strongly connected components in the graph using Tarjan's
    /// algorithm for strongly connected components.
    fn tarjan_scc(&'a self) -> Vec<Vec<usize>> {
        // TODO: Don't do this recursively and use an iterative version of this
        // algorithm instead. There may be cases where the graph is too deep and
        // overflows the stack.

        /// Bookkeeping data for each node.
        #[derive(Copy, Clone, Debug)]
        struct NodeData {
            index: Option<usize>,

            /// The smallest index of any node known to be reachable from this
            /// node. If this value is equal to the index of this node, then it
            /// is the root of the strongly connected component.
            lowlink: usize,

            /// `true` if this vertex is currently on the depth-first search
            /// stack.
            on_stack: bool,
        }

        #[derive(Debug)]
        struct Data<'a> {
            index: usize,
            nodes: Vec<NodeData>,
            stack: Vec<usize>,
            sccs: &'a mut Vec<Vec<usize>>,
        }

        fn scc_visit<'a, 'b, G>(v: usize, g: &'a G, data: &'b mut Data)
        where
            G: Neighbors<'a> + 'a,
        {
            if data.nodes[v].index.is_some() {
                // already visited
                return;
            }

            let v_index = data.index;
            data.nodes[v].index = Some(v_index);
            data.nodes[v].lowlink = v_index;
            data.nodes[v].on_stack = true;
            data.stack.push(v);
            data.index += 1;

            for w in g.outgoing(v) {
                match data.nodes[w].index {
                    None => {
                        scc_visit(w, g, data);
                        data.nodes[v].lowlink = cmp::min(
                            data.nodes[v].lowlink,
                            data.nodes[w].lowlink,
                        );
                    }
                    Some(w_index) => {
                        if data.nodes[w].on_stack {
                            // Successor w is in stack S and hence in the
                            // current SCC
                            let v_lowlink = &mut data.nodes[v].lowlink;
                            *v_lowlink = cmp::min(*v_lowlink, w_index);
                        }
                    }
                }
            }

            // If v is a root node, pop the stack and generate an SCC
            if let Some(v_index) = data.nodes[v].index {
                if data.nodes[v].lowlink == v_index {
                    let mut cur_scc = Vec::new();
                    loop {
                        let w = data.stack.pop().unwrap();
                        data.nodes[w].on_stack = false;
                        cur_scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    data.sccs.push(cur_scc);
                }
            }
        }

        let mut sccs = Vec::new();
        {
            let map = vec![
                NodeData {
                    index: None,
                    lowlink: !0,
                    on_stack: false
                };
                self.node_count()
            ];

            let mut data = Data {
                index: 0,
                nodes: map,
                stack: Vec::new(),
                sccs: &mut sccs,
            };

            for i in 0..self.node_count() {
                scc_visit(i, self, &mut data);
            }
        }

        sccs
    }

    /// Traverses the graph in topological order.
    ///
    /// The function `visit` is called for each node that is to be visited. If
    /// the visitor function returns `true`, then its child nodes may be
    /// visited. If it returns `false`, then its child nodes will not be
    /// visited. This is useful if a resource is determined to be
    /// unchanged, obviating the need to do additional work.
    fn traverse<F, Error>(
        &'a self,
        visit: F,
        threads: usize,
        reverse: bool,
    ) -> Result<(), Vec<(usize, Error)>>
    where
        Self: Sync + Visitable<bool> + NodeIndexable<'a>,
        Self::Node: Sync,
        Self::Edge: Sync,
        Self::Map: Send + Sync,
        F: Fn(usize, usize, &Self::Node) -> Result<bool, Error> + Send + Sync,
        Error: Send,
    {
        let threads = cmp::max(threads, 1);

        let state = TraversalState::new(self, reverse);

        crossbeam::scope(|scope| {
            let state = &state;
            let visit = &visit;

            for tid in 0..threads {
                scope.spawn(move || {
                    traversal_worker(self, tid, threads, state, visit, reverse)
                });
            }
        });

        let errors = state.errors.into_inner().unwrap();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Returns an iterator over the nodes in the graph, depth first.
    fn dfs<I>(&'a self, roots: I) -> DepthFirstSearch<'a, Self>
    where
        I: Iterator<Item = usize>,
        Self: Visitable<()>,
    {
        DepthFirstSearch::new(self, roots)
    }
}

/// Graph traversal worker thread.
fn traversal_worker<'a, G, F, Error>(
    g: &'a G,
    tid: usize,
    threads: usize,
    state: &TraversalState<G, Error>,
    visit: &F,
    reverse: bool,
) where
    G: Neighbors<'a> + NodeIndexable<'a> + Visitable<bool> + Algo<'a>,
    F: Fn(usize, usize, &G::Node) -> Result<bool, Error> + Sync,
    Error: Send,
{
    use std::iter::repeat;

    while let Some(index) = state.queue.pop() {
        // Only call the visitor function if:
        //  1. This node has no incoming edges, or
        //  2. Any of its incoming nodes have had its visitor function
        //     called.
        //
        // Although the entire graph is traversed (unless an error occurs),
        // we may only call the visitor function on a subset of
        // it.
        let do_visit = {
            let mut incoming = g.neighbors(index, !reverse);
            let visited = state.visited.lock().unwrap();
            util::empty_or_any(&mut incoming, |p| {
                visited.is_visited(&p) == Some(&true)
            })
        };

        let keep_going = if do_visit {
            visit(tid, index, g.from_index(index))
        } else {
            Ok(false)
        };

        let mut visited = state.visited.lock().unwrap();

        match keep_going {
            Ok(keep_going) => visited.visit(index, keep_going),
            Err(err) => {
                let mut errors = state.errors.lock().unwrap();
                errors.push((index, err));
                visited.visit(index, false);

                // If we're the last node to be processed, shutdown all
                // threads.
                if state.active.fetch_sub(1, Ordering::Relaxed) == 1 {
                    state.queue.push_many(repeat(None).take(threads));
                }

                // In case of error, do not traverse child nodes. Nothing
                // that depends on this node should be visited.
                continue;
            }
        };

        // Only visit a node if that node's incoming nodes have all been
        // visited. There might be more efficient ways to do this.
        for neigh in g.neighbors(index, reverse) {
            if visited.is_visited(&neigh).is_none()
                && g.neighbors(neigh, !reverse)
                    .all(|p| visited.is_visited(&p).is_some())
            {
                state.active.fetch_add(1, Ordering::Relaxed);
                state.queue.push(Some(neigh));
            }
        }

        // If we're the last node to be processed, shutdown all threads.
        if state.active.fetch_sub(1, Ordering::Relaxed) == 1 {
            state.queue.push_many(repeat(None).take(threads));
        }
    }
}

pub struct RootNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    graph: &'a G,
    nodes: <G as Nodes<'a>>::Iter,
}

impl<'a, G> RootNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    pub fn new(graph: &'a G) -> RootNodes<'a, G> {
        RootNodes {
            graph,
            nodes: graph.nodes(),
        }
    }
}

impl<'a, G> Iterator for RootNodes<'a, G>
where
    G: Nodes<'a> + Neighbors<'a> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.incoming(i).next().is_none() {
                return Some(i);
            }
        }

        None
    }
}

pub struct NonRootNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    graph: &'a G,
    nodes: <G as Nodes<'a>>::Iter,
}

impl<'a, G> NonRootNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    pub fn new(graph: &'a G) -> NonRootNodes<'a, G> {
        NonRootNodes {
            graph,
            nodes: graph.nodes(),
        }
    }
}

impl<'a, G> Iterator for NonRootNodes<'a, G>
where
    G: Nodes<'a> + Neighbors<'a> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.incoming(i).next().is_some() {
                return Some(i);
            }
        }

        None
    }
}

pub struct TerminalNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    graph: &'a G,
    nodes: <G as Nodes<'a>>::Iter,
}

impl<'a, G> TerminalNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    pub fn new(graph: &'a G) -> TerminalNodes<'a, G> {
        TerminalNodes {
            graph,
            nodes: graph.nodes(),
        }
    }
}

impl<'a, G> Iterator for TerminalNodes<'a, G>
where
    G: Nodes<'a> + Neighbors<'a> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.outgoing(i).next().is_none() {
                return Some(i);
            }
        }

        None
    }
}

pub struct NonTerminalNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    graph: &'a G,
    nodes: <G as Nodes<'a>>::Iter,
}

impl<'a, G> NonTerminalNodes<'a, G>
where
    G: Nodes<'a> + 'a,
{
    pub fn new(graph: &'a G) -> NonTerminalNodes<'a, G> {
        NonTerminalNodes {
            graph,
            nodes: graph.nodes(),
        }
    }
}

impl<'a, G> Iterator for NonTerminalNodes<'a, G>
where
    G: Nodes<'a> + Neighbors<'a> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.outgoing(i).next().is_some() {
                return Some(i);
            }
        }

        None
    }
}

pub struct DepthFirstSearch<'a, G>
where
    G: Visitable<()> + 'a,
{
    graph: &'a G,
    stack: Vec<usize>,
    visited: G::Map,
}

impl<'a, G> DepthFirstSearch<'a, G>
where
    G: Visitable<()> + 'a,
{
    pub fn new<I>(graph: &'a G, roots: I) -> DepthFirstSearch<'a, G>
    where
        I: Iterator<Item = usize>,
    {
        DepthFirstSearch {
            graph,
            stack: roots.collect(),
            visited: graph.visit_map(),
        }
    }
}

impl<'a, G> Iterator for DepthFirstSearch<'a, G>
where
    G: Neighbors<'a> + Visitable<()> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            for succ in self.graph.outgoing(node) {
                if self.visited.visit(succ, ()).is_none() {
                    self.stack.push(succ);
                }
            }

            return Some(node);
        }

        None
    }
}

pub trait Graphviz {
    /// GraphViz formatting of the graph.
    fn graphviz(&self, f: &mut io::Write) -> Result<(), io::Error>;
}
