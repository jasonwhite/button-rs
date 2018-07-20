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
use std::hash::Hash;
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crossbeam;

use indexmap::map::{self, IndexMap};

use util::{empty_or_any, RandomQueue};

// Use a random queue. On average, this seems to have better CPU utilization.
type Queue<T> = RandomQueue<Option<T>>;

pub trait NodeTrait: Ord + Hash {}
impl<N> NodeTrait for N where N: Ord + Hash {}

type Visited = HashMap<usize, bool>;

#[derive(
    Serialize,
    Deserialize,
    Default,
    Debug,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
)]
pub struct NodeNeighbors {
    pub incoming: Vec<usize>,
    pub outgoing: Vec<usize>,
}

impl NodeNeighbors {
    pub fn new() -> NodeNeighbors {
        NodeNeighbors {
            incoming: Vec::new(),
            outgoing: Vec::new(),
        }
    }
}

struct TraversalState<E>
where
    E: Send,
{
    // List of errors that occurred during the traversal.
    pub errors: Mutex<Vec<(usize, E)>>,

    // Nodes that have been visited. The value in this map indicates
    // whether or not the visitor function was called on it.
    pub visited: Mutex<Visited>,

    // Queue of node indices. All the items in the queue have the property of
    // not depending on each other.
    pub queue: Queue<usize>,

    // Keeps a count of the number of nodes being processed (or waiting to
    // be processed). When this reaches 0, we know there is no more
    // work to do.
    pub active: AtomicUsize,
}

impl<E> TraversalState<E>
where
    E: Send,
{
    pub fn new<I>(roots: I) -> TraversalState<E>
    where
        I: Iterator<Item = Option<usize>>,
    {
        let queue = Queue::new();
        let active = queue.push_many(roots);

        TraversalState {
            errors: Mutex::new(Vec::new()),
            visited: Mutex::new(Visited::new()),
            queue,
            active: AtomicUsize::new(active),
        }
    }
}

/// Directed graph.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Graph<N, E>
where
    N: NodeTrait,
{
    nodes: IndexMap<N, NodeNeighbors>,
    edges: IndexMap<(usize, usize), E>,
}

impl<N, E> Graph<N, E>
where
    N: NodeTrait,
{
    /// Creates a new `Graph`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `Graph` with an estimated capacity.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Graph {
            nodes: IndexMap::with_capacity(nodes),
            edges: IndexMap::with_capacity(edges),
        }
    }

    /// Returns the current node and edge capacity of the graph.
    pub fn capacity(&self) -> (usize, usize) {
        (self.nodes.capacity(), self.edges.capacity())
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the index of the given node if it exists.
    pub fn node_index(&self, n: &N) -> Option<usize> {
        self.nodes.get_full(n).map(|x| x.0)
    }

    /// Translates an index to a node. Panics if the index is not in the graph.
    pub fn node(&self, index: usize) -> &N {
        self.nodes.get_index(index).unwrap().0
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Removes all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }

    /// Add node `n` to the graph. Returns the index of the node.
    pub fn add_node(&mut self, n: N) -> usize {
        let entry = self.nodes.entry(n);
        let index = entry.index();
        entry.or_insert(NodeNeighbors::new());
        index
    }

    /// Returns `true` if the node exists in the graph.
    pub fn contains_node(&self, n: &N) -> bool {
        self.nodes.contains_key(n)
    }

    /// Adds an edge to the graph. Returns the old weight of the edge if it
    /// already existed.
    pub fn add_edge(&mut self, a: usize, b: usize, weight: E) -> Option<E> {
        let old = self.edges.insert((a, b), weight);
        if old.is_some() {
            old
        } else {
            // New edge. It needs to be inserted into the node
            if let Some((_, v)) = self.nodes.get_index_mut(a) {
                v.outgoing.push(b);
            }

            if a != b {
                if let Some((_, v)) = self.nodes.get_index_mut(b) {
                    v.incoming.push(a);
                }
            }

            None
        }
    }

    /// Returns `true` if the edge exists.
    pub fn contains_edge(&self, a: usize, b: usize) -> bool {
        self.edges.contains_key(&(a, b))
    }

    /// An iterator over the nodes of the graph.
    pub fn nodes(&self) -> Nodes<N> {
        Nodes {
            iter: self.nodes.keys(),
        }
    }

    /// Returns an iterator over all edges in the graph.
    pub fn edges(&self) -> Edges<E> {
        Edges {
            iter: self.edges.iter(),
        }
    }

    /// Returns an iterator over all the outgoing edges for the given node.
    /// Panics if the index does not exist.
    pub fn outgoing(&self, index: usize) -> Neighbors {
        Neighbors {
            iter: self.nodes.get_index(index).unwrap().1.outgoing.iter(),
        }
    }

    /// Returns an iterator over all the incoming edges for the given node.
    /// Panics if the index does not exist.
    pub fn incoming(&self, index: usize) -> Neighbors {
        Neighbors {
            iter: self.nodes.get_index(index).unwrap().1.incoming.iter(),
        }
    }

    /// Returns the number of outgoing edges for the given node.
    ///
    /// Returns `None` if the node does not exist.
    pub fn outgoing_count(&self, index: usize) -> Option<usize> {
        self.nodes.get_index(index).map(|(_, v)| v.outgoing.len())
    }

    /// Returns the number of incoming edges for the given node.
    ///
    /// Returns `None` if the node does not exist.
    pub fn incoming_count(&self, index: usize) -> Option<usize> {
        self.nodes.get_index(index).map(|(_, v)| v.incoming.len())
    }

    /// Iterator over all "roots" of the graph. That is, all the node indices
    /// that have no incoming edges.
    ///
    /// This is an O(n) operation.
    pub fn root_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, &'a N)> + 'a {
        NodeFilter::new(self.nodes.iter(), |neighbors| {
            neighbors.incoming.is_empty()
        })
    }

    /// Iterator over all nodes with one or more incoming edges. This includes
    /// all non-root nodes.
    pub fn non_root_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, &'a N)> + 'a {
        NodeFilter::new(self.nodes.iter(), |neighbors| {
            !neighbors.incoming.is_empty()
        })
    }

    /// Iterator over all terminal nodes of the graph. That is, all node indices
    /// that have no *outgoing* edges.
    ///
    /// This is an O(n) operation.
    pub fn terminal_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, &'a N)> + 'a {
        NodeFilter::new(self.nodes.iter(), |neighbors| {
            neighbors.outgoing.is_empty()
        })
    }

    /// Iterator over all terminal nodes of the graph. That is, all node indices
    /// that have no *outgoing* edges.
    ///
    /// This is an O(n) operation.
    pub fn non_terminal_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, &'a N)> + 'a {
        NodeFilter::new(self.nodes.iter(), |neighbors| {
            !neighbors.outgoing.is_empty()
        })
    }

    /// Given an index, translate it to an index in the other graph. Returns
    /// `None` if the index does not exist in the other graph.
    pub fn translate_index(
        &self,
        index: usize,
        other: &Graph<N, E>,
    ) -> Option<usize> {
        other.node_index(self.node(index))
    }

    /// Traverses the graph in topological order.
    ///
    /// The function `visit` is called for each node that is to be visited. If
    /// the visitor function returns `true`, then its child nodes may be
    /// visited. If it returns `false`, then its child nodes will not be
    /// visited. This is useful if a resource is determined to be
    /// unchanged, obviating the need to do additional work.
    pub fn traverse<F, Error>(
        &self,
        visit: F,
        threads: usize,
    ) -> Result<(), Vec<(usize, Error)>>
    where
        N: Sync,
        E: Sync,
        F: Fn(usize, &N) -> Result<bool, Error> + Send + Sync,
        Error: Send,
    {
        // Always use at least one thread.
        let threads = cmp::max(threads, 1);

        let roots = self.root_nodes().map(|x| Some(x.0));
        let state = TraversalState::new(roots);

        crossbeam::scope(|scope| {
            let state = &state;
            let visit = &visit;

            for id in 0..threads {
                scope.spawn(move || {
                    self.traversal_worker(id, threads, state, visit)
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

    /// Graph traversal worker thread.
    fn traversal_worker<F, Error>(
        &self,
        id: usize,
        threads: usize,
        state: &TraversalState<Error>,
        visit: &F,
    ) where
        F: Fn(usize, &N) -> Result<bool, Error> + Sync,
        Error: Send,
    {
        use std::iter::repeat;

        while let Some(node) = state.queue.pop() {
            // Only call the visitor function if:
            //  1. This node has no incoming edges, or
            // 2. Any of its incoming nodes have had its visitor function
            // called.
            //
            // Although the entire graph is traversed (unless an error occurs),
            // we may only call the visitor function on a subset of
            // it.
            let do_visit = {
                let mut incoming = self.incoming(node);
                let visited = state.visited.lock().unwrap();
                empty_or_any(&mut incoming, |p| visited.get(&p) == Some(&true))
            };

            let keep_going = if do_visit {
                visit(id, self.node(node))
            } else {
                Ok(false)
            };

            let mut visited = state.visited.lock().unwrap();

            match keep_going {
                Ok(keep_going) => visited.insert(node, keep_going),
                Err(err) => {
                    let mut errors = state.errors.lock().unwrap();
                    errors.push((node, err));
                    visited.insert(node, false);

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

            for neigh in self.outgoing(node) {
                // Only visit a node if that node's incoming nodes have all been
                // visited. There might be more efficient ways to do this.
                if !visited.contains_key(neigh)
                    && self.incoming(*neigh).all(|p| visited.contains_key(&p))
                {
                    state.active.fetch_add(1, Ordering::Relaxed);
                    state.queue.push(Some(*neigh));
                }
            }

            // If we're the last node to be processed, shutdown all threads.
            if state.active.fetch_sub(1, Ordering::Relaxed) == 1 {
                state.queue.push_many(repeat(None).take(threads));
            }
        }
    }

    /// Returns the strongly connected components in the graph using Tarjan's
    /// algorithm for strongly connected components.
    pub fn tarjan_scc(&self) -> Vec<Vec<usize>> {
        // TODO: Don't do this recursively and use an iterative version of this
        // algorithm instead. There may be cases where the graph is too deep and
        // overflows the stack.

        /// Bookkeeping data for each node.
        #[derive(Copy, Clone, Debug)]
        struct NodeData {
            index: Option<usize>,

            /// The smallest index of any node known to be reachable from this
            /// node. If this value is equal to the index of this node,
            /// then it is the root of the strongly connected component.
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

        fn scc_visit<N, E>(v: usize, g: &Graph<N, E>, data: &mut Data)
        where
            N: NodeTrait,
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
                match data.nodes[*w].index {
                    None => {
                        scc_visit(*w, g, data);
                        data.nodes[v].lowlink = cmp::min(
                            data.nodes[v].lowlink,
                            data.nodes[*w].lowlink,
                        );
                    }
                    Some(w_index) => {
                        if data.nodes[*w].on_stack {
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
}

/// Creates a new empty `Graph`.
impl<N, E> Default for Graph<N, E>
where
    N: NodeTrait,
{
    fn default() -> Self {
        Graph::with_capacity(0, 0)
    }
}

/// Iterator over the nodes in the graph.
pub struct Nodes<'a, N>
where
    N: 'a,
{
    iter: map::Keys<'a, N, NodeNeighbors>,
}

impl<'a, N> Iterator for Nodes<'a, N>
where
    N: 'a + NodeTrait,
{
    type Item = &'a N;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n)
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last()
    }
}

/// Iterator over all of the edges in the graph.
pub struct Edges<'a, E>
where
    E: 'a,
{
    iter: map::Iter<'a, (usize, usize), E>,
}

impl<'a, E> Iterator for Edges<'a, E>
where
    E: 'a,
{
    type Item = (usize, usize, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some((&(a, b), w)) => Some((a, b, w)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|(&(a, b), w)| (a, b, w))
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last().map(|(&(a, b), w)| (a, b, w))
    }
}

pub struct Neighbors<'a> {
    iter: slice::Iter<'a, usize>,
}

impl<'a> Iterator for Neighbors<'a> {
    type Item = &'a usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n)
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last()
    }
}

/// A filter for nodes in the graph.
pub struct NodeFilter<'a, N, P>
where
    N: 'a,
{
    // Current index in the list.
    index: usize,
    iter: map::Iter<'a, N, NodeNeighbors>,
    predicate: P,
}

impl<'a, N, P> NodeFilter<'a, N, P>
where
    N: 'a,
    P: FnMut(&'a NodeNeighbors) -> bool,
{
    pub fn new(
        iter: map::Iter<'a, N, NodeNeighbors>,
        predicate: P,
    ) -> NodeFilter<N, P> {
        NodeFilter {
            index: 0,
            iter,
            predicate,
        }
    }
}

impl<'a, N, P> Iterator for NodeFilter<'a, N, P>
where
    P: FnMut(&'a NodeNeighbors) -> bool,
{
    type Item = (usize, &'a N);

    fn next(&mut self) -> Option<Self::Item> {
        for (node, neighbors) in &mut self.iter {
            let index = self.index;
            self.index += 1;

            if (self.predicate)(neighbors) {
                return Some((index, node));
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }

    // Make counting fast.
    fn count(mut self) -> usize {
        let mut count = 0;
        for x in &mut self.iter {
            count += (self.predicate)(x.1) as usize;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke() {
        let mut g = Graph::new();
        let a = g.add_node("a");
        let b = g.add_node("b");
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(g.node_count(), 2);

        assert_eq!(g.add_edge(a, b, 42), None);
        assert_eq!(g.add_edge(a, b, 1), Some(42));
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_tarjan() {
        // This is Wikipedia's example graph:
        //
        //  O ← 1 ← 2 ⇄ 3
        //  ↓ ↗ ↑   ↑   ↑
        //  4 ← 5 ⇄ 6 ← 7
        //              ↺
        let mut graph = Graph::new();

        // Top row
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        let d = graph.add_node("d");

        // Bottom row
        let e = graph.add_node("e");
        let f = graph.add_node("f");
        let g = graph.add_node("g");
        let h = graph.add_node("h");

        graph.add_edge(a, e, ());
        graph.add_edge(b, a, ());
        graph.add_edge(c, b, ());
        graph.add_edge(c, d, ());
        graph.add_edge(d, c, ());
        graph.add_edge(e, b, ());
        graph.add_edge(f, b, ());
        graph.add_edge(f, e, ());
        graph.add_edge(f, g, ());
        graph.add_edge(g, c, ());
        graph.add_edge(g, f, ());
        graph.add_edge(h, d, ());
        graph.add_edge(h, g, ());
        graph.add_edge(h, h, ());

        let sccs = tarjan_scc(&graph);
        assert_eq!(sccs.len(), 4);

        assert_eq!(sccs[0], vec![1, 4, 0]);
        assert_eq!(sccs[1], vec![3, 2]);
        assert_eq!(sccs[2], vec![6, 5]);
        assert_eq!(sccs[3], vec![7]);
    }
}
