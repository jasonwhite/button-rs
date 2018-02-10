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
use std::cmp::min;
use std::hash::Hash;
use std::slice;

use indexmap::map::{self, IndexMap};

pub trait NodeTrait: Ord + Hash {}
impl<N> NodeTrait for N where N: Ord + Hash {}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct NodeNeighbors {
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

/// Directed graph.
#[derive(Clone, Eq, PartialEq)]
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

    /// Translates an index to a node.
    pub fn node(&self, index: usize) -> Option<&N> {
        self.nodes.get_index(index).map(|x| x.0)
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
    pub fn contains_node(&self, n: N) -> bool {
        self.nodes.contains_key(&n)
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
    pub fn all_edges(&self) -> AllEdges<E> {
        AllEdges {
            iter: self.edges.iter(),
        }
    }

    /// Returns an iterator over all the outgoing edges for the given node. If
    /// the node does not exist, returns an empty iterator.
    pub fn outgoing(&self, index: usize) -> Neighbors {
        Neighbors {
            iter: match self.nodes.get_index(index) {
                Some((_, neighbors)) => neighbors.outgoing.iter(),
                None => [].iter(),
            },
        }
    }

    /// Returns an iterator over all the incoming edges for the given node. If
    /// the node does not exist, returns an empty iterator.
    pub fn incoming(&self, index: usize) -> Neighbors {
        Neighbors {
            iter: match self.nodes.get_index(index) {
                Some((_, neighbors)) => neighbors.incoming.iter(),
                None => [].iter(),
            },
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
    pub fn roots(&self) -> Roots<N> {
        Roots {
            index: 0,
            iter: self.nodes.values(),
        }
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
    N: 'a + NodeTrait,
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
pub struct AllEdges<'a, E>
where
    E: 'a,
{
    iter: map::Iter<'a, (usize, usize), E>,
}

impl<'a, E> Iterator for AllEdges<'a, E>
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

/// Iterator over the roots of the graph.
pub struct Roots<'a, N>
where
    N: 'a,
{
    // Current index in the list.
    index: usize,
    iter: map::Values<'a, N, NodeNeighbors>,
}

impl<'a, N> Iterator for Roots<'a, N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(neighbors) = self.iter.next() {
            let index = self.index;
            self.index += 1;

            if neighbors.incoming.len() == 0 {
                return Some(index);
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

/// Returns the strongly connected components in the graph using Tarjan's
/// algorithm for strongly connected components.
pub fn tarjan_scc<N, E>(g: &Graph<N, E>) -> Vec<Vec<usize>>
where
    N: NodeTrait,
{
    // TODO: Don't do this recursively and use an iterative version of this
    // algorithm instead. There may be cases where the graph is too deep and
    // overflows the stack.

    /// Bookkeeping data for each node.
    #[derive(Copy, Clone, Debug)]
    struct NodeData {
        index: Option<usize>,

        /// The smallest index of any node known to be reachable from this
        /// node. If this value is equal to the index of this node,
        /// then it is the root of the strongly conected component.
        lowlink: usize,

        /// `true` if this vertex is curently on the depth-first search stack.
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
                    data.nodes[v].lowlink =
                        min(data.nodes[v].lowlink, data.nodes[*w].lowlink);
                }
                Some(w_index) => {
                    if data.nodes[*w].on_stack {
                        // Successor w is in stack S and hence in the current
                        // SCC
                        let v_lowlink = &mut data.nodes[v].lowlink;
                        *v_lowlink = min(*v_lowlink, w_index);
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
            g.node_count()
        ];

        let mut data = Data {
            index: 0,
            nodes: map,
            stack: Vec::new(),
            sccs: &mut sccs,
        };

        for i in 0..g.node_count() {
            scc_visit(i, g, &mut data);
        }
    }

    sccs
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
