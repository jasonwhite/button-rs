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
use std::iter;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crossbeam;

use util;

use super::index::{EdgeIndex, IndexSet, NodeIndex};

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

    /// Returns the number of edges in the graph.
    fn edge_count(&self) -> usize;
}

/// A graph that is indexable.
pub trait Indexable<'a>: GraphBase
where
    Self::Node: 'a,
{
    fn try_node_from_index(
        &'a self,
        index: NodeIndex,
    ) -> Option<&'a Self::Node>;

    fn try_edge_from_index(
        &'a self,
        index: EdgeIndex,
    ) -> Option<((NodeIndex, NodeIndex), &'a Self::Edge)>;

    /// Converts a node index into a node.
    ///
    /// Panics if the index does not exist.
    fn node_from_index(&'a self, index: NodeIndex) -> &'a Self::Node {
        self.try_node_from_index(index).unwrap()
    }

    /// Converts an edge index into a pair.
    ///
    /// Panics if the index does not exist.
    fn edge_from_index(
        &'a self,
        index: EdgeIndex,
    ) -> ((NodeIndex, NodeIndex), &'a Self::Edge) {
        self.try_edge_from_index(index).unwrap()
    }

    /// Converts a node to an index if it exists.
    ///
    /// Returns `None` if the node does not exist in the graph.
    fn node_to_index(&self, node: &Self::Node) -> Option<NodeIndex>;

    /// Converts an edge index into a pair.
    ///
    /// Returns `None` if the edge does not exist in the graph.
    fn edge_to_index(&self, edge: &(NodeIndex, NodeIndex))
        -> Option<EdgeIndex>;

    /// Returns `true` if the node exists in the graph.
    fn contains_node(&self, n: &Self::Node) -> bool {
        self.node_to_index(n).is_some()
    }

    /// Returns `true` if the node exists in the graph.
    fn contains_node_index(&'a self, index: NodeIndex) -> bool {
        self.try_node_from_index(index).is_some()
    }

    /// Returns `true` if the edge exists in the graph.
    fn contains_edge(&self, e: &(NodeIndex, NodeIndex)) -> bool {
        self.edge_to_index(e).is_some()
    }

    /// Returns `true` if the node exists in the graph.
    fn contains_edge_index(&'a self, index: EdgeIndex) -> bool {
        self.try_edge_from_index(index).is_some()
    }
}

/// Trait for iterating over the neighbors of nodes.
pub trait Neighbors<'a>: GraphBase {
    type Neighbors: Iterator<Item = (NodeIndex, EdgeIndex)>;

    /// Returns an iterator over all the incoming edges for the given node.
    ///
    /// Panics if the node index does not exist.
    fn incoming(&'a self, node: NodeIndex) -> Self::Neighbors;

    /// Returns an iterator over all the outgoing edges for the given node.
    ///
    /// Panics if the node index does not exist.
    fn outgoing(&'a self, node: NodeIndex) -> Self::Neighbors;

    /// Returns the neighbors. If `reverse` is `false`, returns the outgoing
    /// neighbors. Otherwise, if `reverse` is `true`, returns the incoming
    /// neighbors.
    ///
    /// Panics if the node index does not exist.
    fn neighbors(&'a self, node: NodeIndex, reverse: bool) -> Self::Neighbors {
        if reverse {
            self.incoming(node)
        } else {
            self.outgoing(node)
        }
    }

    /// Returns true if the given node is a root node (i.e., it has no incoming
    /// edges).
    ///
    /// Panics if the node index does not exist.
    fn is_root_node(&'a self, node: NodeIndex) -> bool {
        self.incoming(node).next().is_none()
    }

    /// Returns true if the given node is a terminal node (i.e., it has no
    /// outgoing edges).
    ///
    /// Panics if the node index does not exist.
    fn is_terminal_node(&'a self, node: NodeIndex) -> bool {
        self.outgoing(node).next().is_none()
    }
}

/// Trait for iterating over the nodes in the graph.
pub trait Nodes<'a>: GraphBase
where
    Self::Node: 'a,
{
    type Iter: Iterator<Item = NodeIndex>;

    /// Returns an iterator over the nodes in the graph.
    fn nodes(&'a self) -> Self::Iter;
}

/// Trait for iterating over the edges in the graph.
pub trait Edges<'a>: GraphBase
where
    Self::Edge: 'a,
{
    type Iter: Iterator<Item = EdgeIndex>;

    /// Returns an iterator over the edges in the graph.
    fn edges(&'a self) -> Self::Iter;
}

/// A mapping for storing visited status.
pub trait VisitMap<N, T> {
    /// Marks the node as visited. Returns the previous visited state if any.
    fn visit(&mut self, node: N, value: T) -> Option<T>;

    /// Returns `Some(&T)` if this node has been visited. Returns `None` if it
    /// hasn't been visited.
    fn get(&self, node: &N) -> Option<&T>;

    /// Returns a mutable reference to the node if it has been visited. Returns
    /// `None` if it hasn't been visited.
    fn get_mut(&mut self, node: &N) -> Option<&mut T>;

    fn is_visited(&self, node: &N) -> bool {
        self.get(node).is_some()
    }

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

    fn get(&self, node: &N) -> Option<&T> {
        self.get(node)
    }

    fn get_mut(&mut self, node: &N) -> Option<&mut T> {
        self.get_mut(node)
    }

    fn clear(&mut self) {
        HashMap::clear(self)
    }
}

impl<T> VisitMap<NodeIndex, T> for Vec<Option<T>> {
    fn visit(&mut self, node: NodeIndex, value: T) -> Option<T> {
        let i: usize = node.into();

        // Ensure the vector is large enough.
        let mut j = self.len();
        while j <= i {
            self.push(None);
            j += 1;
        }

        mem::replace(&mut self[i], Some(value))
    }

    fn get(&self, node: &NodeIndex) -> Option<&T> {
        let i: usize = (*node).into();
        <[Option<T>]>::get(self.as_slice(), i).and_then(|v| v.as_ref())
    }

    fn get_mut(&mut self, node: &NodeIndex) -> Option<&mut T> {
        let i: usize = (*node).into();
        <[Option<T>]>::get_mut(self.as_mut_slice(), i).and_then(|v| v.as_mut())
    }

    fn clear(&mut self) {
        let len = self.len();

        Vec::clear(self);

        for _ in 0..len {
            self.push(None);
        }
    }
}

/// A mapping for storing visited status.
pub trait VisitSet<N> {
    /// Marks the node as visited. Returns true if the node hasn't been visited
    /// before.
    fn visit(&mut self, node: N) -> bool;

    /// Returns true if the node has been visited.
    fn is_visited(&self, node: &N) -> bool;

    /// Marks all nodes as unvisited.
    fn clear(&mut self);
}

impl VisitSet<NodeIndex> for IndexSet<NodeIndex> {
    fn visit(&mut self, node: NodeIndex) -> bool {
        self.insert(node)
    }

    fn is_visited(&self, node: &NodeIndex) -> bool {
        self.contains(node)
    }

    fn clear(&mut self) {
        IndexSet::clear(self)
    }
}

/// A graph that can track the visited status of its nodes.
pub trait Visitable<T> {
    type Map: VisitMap<NodeIndex, T>;

    /// Creates a new visit map.
    fn visit_map(&self) -> Self::Map;
}

/// The state passed to worker threads when traversing the graph.
struct TraversalState<G, E>
where
    G: Visitable<bool>,
    E: Send,
{
    pub threads: usize,

    // List of errors that occurred during the traversal.
    pub errors: Mutex<Vec<(NodeIndex, E)>>,

    // Nodes that have been visited. The value in this map indicates
    // whether or not the visitor function was called on it.
    pub visited: Mutex<G::Map>,

    // Queue of node indices. All the items in the queue have the property of
    // not depending on each other.
    pub queue: Queue<NodeIndex>,

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
    pub fn new(
        graph: &'a G,
        reverse: bool,
        threads: usize,
    ) -> TraversalState<G, E> {
        let queue = Queue::new();

        let active = if reverse {
            queue.push_many(graph.terminal_nodes().map(Some))
        } else {
            queue.push_many(graph.root_nodes().map(Some))
        };

        TraversalState {
            threads,
            errors: Mutex::new(Vec::new()),
            visited: Mutex::new(graph.visit_map()),
            queue,
            active: AtomicUsize::new(active),
        }
    }

    /// Signals all threads to stop their work after they finish what they're
    /// currently doing.
    pub fn shutdown(&self) {
        self.queue.push_many(iter::repeat(None).take(self.threads));
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TarjanNodeData {
    index: usize,

    /// The smallest index of any node known to be reachable from this
    /// node. If this value is equal to the index of this node, then it
    /// is the root of the strongly connected component.
    lowlink: usize,

    /// `true` if this vertex is currently on the depth-first search
    /// stack.
    on_stack: bool,
}

pub struct Diff {
    pub right_only_nodes: IndexSet<NodeIndex>,
    pub left_only_nodes: IndexSet<NodeIndex>,
    pub right_only_edges: IndexSet<EdgeIndex>,
    pub left_only_edges: IndexSet<EdgeIndex>,
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
    fn tarjan_scc(&'a self) -> Vec<Vec<NodeIndex>>
    where
        Self: Visitable<TarjanNodeData>,
    {
        // TODO: Don't do this recursively and use an iterative version of this
        // algorithm instead. There may be cases where the graph is too deep and
        // overflows the stack.

        #[derive(Debug)]
        struct Data<'a, M> {
            index: usize,
            nodes: M,
            stack: Vec<NodeIndex>,
            sccs: &'a mut Vec<Vec<NodeIndex>>,
        }

        fn scc_visit<'a, 'b, G>(
            v: NodeIndex,
            g: &'a G,
            data: &'b mut Data<G::Map>,
        ) where
            G: Neighbors<'a> + Visitable<TarjanNodeData> + 'a,
        {
            if data.nodes.is_visited(&v) {
                return;
            }

            let v_index = data.index;

            data.nodes.visit(
                v,
                TarjanNodeData {
                    index: v_index,
                    lowlink: v_index,
                    on_stack: true,
                },
            );

            data.stack.push(v);
            data.index += 1;

            for (w, _) in g.outgoing(v) {
                match data.nodes.get(&w).map(|n| n.index) {
                    None => {
                        scc_visit(w, g, data);

                        data.nodes.get_mut(&v).unwrap().lowlink = cmp::min(
                            data.nodes.get(&v).unwrap().lowlink,
                            data.nodes.get(&w).unwrap().lowlink,
                        );
                    }
                    Some(w_index) => {
                        if data.nodes.get(&w).unwrap().on_stack {
                            // Successor w is in stack and hence in the current
                            // SCC.
                            let v_lowlink =
                                &mut data.nodes.get_mut(&v).unwrap().lowlink;
                            *v_lowlink = cmp::min(*v_lowlink, w_index);
                        }
                    }
                }
            }

            if let Some(v_index) = data.nodes.get(&v).map(|n| n.index) {
                if data.nodes.get(&v).unwrap().lowlink == v_index {
                    let mut cur_scc = Vec::new();

                    loop {
                        let w = data.stack.pop().unwrap();
                        data.nodes.get_mut(&w).unwrap().on_stack = false;
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
            let mut data = Data {
                index: 0,
                nodes: self.visit_map(),
                stack: Vec::new(),
                sccs: &mut sccs,
            };

            for node in self.nodes() {
                scc_visit(node, self, &mut data);
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
    ) -> Result<(), Vec<(NodeIndex, Error)>>
    where
        Self: Sync + Visitable<bool> + Indexable<'a>,
        Self::Node: Sync,
        Self::Edge: Sync,
        Self::Map: Send + Sync,
        F: Fn(usize, NodeIndex, &Self::Node) -> Result<bool, Error>
            + Send
            + Sync,
        Error: Send,
    {
        let threads = cmp::max(threads, 1);

        let state = TraversalState::new(self, reverse, threads);

        crossbeam::scope(|scope| {
            let state = &state;
            let visit = &visit;

            for tid in 0..threads {
                scope.spawn(move || {
                    traversal_worker(self, tid, state, visit, reverse)
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
        I: Iterator<Item = NodeIndex>,
    {
        DepthFirstSearch::new(self, roots)
    }

    /// Finds nodes that are only present in this graph, not the other.
    ///
    /// Computes in `O(|V|)` time.
    fn unique_nodes<G>(&'a self, other: &'a G) -> IndexSet<NodeIndex>
    where
        Self: Indexable<'a>,
        G: GraphBase<Node = Self::Node, Edge = Self::Edge> + Indexable<'a>,
    {
        let mut nodes = IndexSet::new();

        for index in self.nodes() {
            if !other.contains_node(self.node_from_index(index)) {
                nodes.insert(index);
            }
        }

        nodes
    }

    /// Finds edges that are only present in this graph, not the other.
    ///
    /// Computes in `O(|E|)` time.
    fn unique_edges<G>(&'a self, other: &'a G) -> IndexSet<EdgeIndex>
    where
        Self: Indexable<'a> + Edges<'a>,
        G: GraphBase<Node = Self::Node, Edge = Self::Edge> + Indexable<'a>,
    {
        let mut edges = IndexSet::new();

        for index in self.edges() {
            let (from, to) = self.edge_from_index(index).0;

            // Translate to indices that the other graph understands.
            let from = other.node_to_index(self.node_from_index(from));
            let to = other.node_to_index(self.node_from_index(to));

            if let (Some(from), Some(to)) = (from, to) {
                if !other.contains_edge(&(from, to)) {
                    edges.insert(index);
                }
            } else {
                edges.insert(index);
            }
        }

        edges
    }

    /// Diffs this graph with another graph.
    fn diff<G>(&'a self, other: &'a G) -> Diff
    where
        Self: Indexable<'a> + Edges<'a>,
        G: GraphBase<Node = Self::Node, Edge = Self::Edge>
            + Algo<'a>
            + Edges<'a>
            + Indexable<'a>,
    {
        Diff {
            left_only_nodes: self.unique_nodes(other),
            right_only_nodes: other.unique_nodes(self),
            left_only_edges: self.unique_edges(other),
            right_only_edges: other.unique_edges(self),
        }
    }
}

/// Graph traversal worker thread.
fn traversal_worker<'a, G, F, Error>(
    g: &'a G,
    tid: usize,
    state: &TraversalState<G, Error>,
    visit: &F,
    reverse: bool,
) where
    G: Neighbors<'a> + Indexable<'a> + Visitable<bool> + Algo<'a>,
    F: Fn(usize, NodeIndex, &G::Node) -> Result<bool, Error> + Sync,
    Error: Send,
{
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
            util::empty_or_any(&mut incoming, |(p, _)| {
                visited.get(&p) == Some(&true)
            })
        };

        let keep_going = if do_visit {
            visit(tid, index, g.node_from_index(index))
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
                    state.shutdown();
                }

                // In case of error, do not traverse child nodes. Nothing
                // that depends on this node should be visited.
                continue;
            }
        };

        // Only visit a node if that node's incoming nodes have all been
        // visited. There might be more efficient ways to do this.
        for (neigh, _) in g.neighbors(index, reverse) {
            if !visited.is_visited(&neigh)
                && g.neighbors(neigh, !reverse)
                    .all(|(p, _)| visited.is_visited(&p))
            {
                state.active.fetch_add(1, Ordering::Relaxed);
                state.queue.push(Some(neigh));
            }
        }

        // If we're the last node to be processed, shutdown all threads.
        if state.active.fetch_sub(1, Ordering::Relaxed) == 1 {
            state.shutdown();
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
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.is_root_node(i) {
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
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if !self.graph.is_root_node(i) {
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
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if self.graph.is_terminal_node(i) {
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
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.nodes.next() {
            if !self.graph.is_terminal_node(i) {
                return Some(i);
            }
        }

        None
    }
}

pub struct DepthFirstSearch<'a, G: 'a> {
    graph: &'a G,
    stack: Vec<NodeIndex>,
    visited: IndexSet<NodeIndex>,
}

impl<'a, G: 'a> DepthFirstSearch<'a, G> {
    pub fn new<I>(graph: &'a G, roots: I) -> DepthFirstSearch<'a, G>
    where
        I: Iterator<Item = NodeIndex>,
    {
        DepthFirstSearch {
            graph,
            stack: roots.collect(),
            visited: IndexSet::new(),
        }
    }
}

impl<'a, G> Iterator for DepthFirstSearch<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        for (succ, _) in self.graph.outgoing(node) {
            if self.visited.visit(succ) {
                self.stack.push(succ);
            }
        }

        Some(node)
    }
}

pub trait Graphviz {
    /// GraphViz formatting of the graph.
    fn graphviz(&self, f: &mut io::Write) -> Result<(), io::Error>;
}
