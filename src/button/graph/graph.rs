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
use std::hash::Hash;
use std::ops;
use std::slice;

use indexmap::map::{self, IndexMap};

use super::visit::{
    Algo, Edges, GraphBase, Neighbors, NodeIndexable, Nodes, Visitable,
};

pub trait NodeTrait: Ord + Hash {}
impl<N> NodeTrait for N where N: Ord + Hash {}

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

/// Directed graph.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Graph<N, E>
where
    N: NodeTrait,
{
    nodes: IndexMap<N, NodeNeighbors>,
    edges: IndexMap<(usize, usize), E>,
}

impl<N, E> GraphBase for Graph<N, E>
where
    N: NodeTrait,
{
    type Node = N;
    type Edge = E;

    fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl<'a, N, E> NodeIndexable<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
{
    fn from_index(&'a self, index: usize) -> &'a Self::Node {
        self.nodes.get_index(index).unwrap().0
    }

    fn to_index(&self, node: &Self::Node) -> Option<usize> {
        self.nodes.get_full(node).map(|x| x.0)
    }
}

impl<'a, N, E> Nodes<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
{
    type Iter = ops::Range<usize>;

    fn nodes(&'a self) -> Self::Iter {
        (0..self.node_count())
    }
}

impl<'a, N, E> Edges<'a> for Graph<N, E>
where
    N: NodeTrait,
    E: 'a,
{
    type Iter = EdgesIter<'a, E>;

    fn edges(&'a self) -> Self::Iter {
        EdgesIter {
            iter: self.edges.iter(),
        }
    }
}

impl<'a, N, E> Neighbors<'a> for Graph<N, E>
where
    N: NodeTrait,
{
    type Neighbors = NeighborsIter<'a>;

    fn incoming(&'a self, node: usize) -> Self::Neighbors {
        NeighborsIter {
            iter: self.nodes.get_index(node).unwrap().1.incoming.iter(),
        }
    }

    fn outgoing(&'a self, node: usize) -> Self::Neighbors {
        NeighborsIter {
            iter: self.nodes.get_index(node).unwrap().1.outgoing.iter(),
        }
    }
}

/// Be able to use algorithms with this graph.
impl<'a, N, E> Algo<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
    E: 'a,
{
}

impl<N, E, T> Visitable<T> for Graph<N, E>
where
    N: NodeTrait,
{
    type Map = Vec<Option<T>>;

    fn visit_map(&self) -> Self::Map {
        let mut map = Vec::with_capacity(self.node_count());

        for _ in 0..self.node_count() {
            map.push(None)
        }

        map
    }
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

    /// Add node `n` to the graph. Returns the index of the node.
    pub fn add_node(&mut self, n: N) -> usize {
        let entry = self.nodes.entry(n);
        let index = entry.index();
        entry.or_insert(NodeNeighbors::new());
        index
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

    /// Given an index, translate it to an index in the other graph. Returns
    /// `None` if the index does not exist in the other graph.
    ///
    /// Pancis if the index does not exist in this graph.
    pub fn translate_index(
        &self,
        index: usize,
        other: &Graph<N, E>,
    ) -> Option<usize> {
        other.to_index(self.from_index(index))
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

/// Iterator over all of the edges in the graph.
pub struct EdgesIter<'a, E>
where
    E: 'a,
{
    iter: map::Iter<'a, (usize, usize), E>,
}

impl<'a, E> Iterator for EdgesIter<'a, E>
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

pub struct NeighborsIter<'a> {
    iter: slice::Iter<'a, usize>,
}

impl<'a> Iterator for NeighborsIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).cloned()
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last().cloned()
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
