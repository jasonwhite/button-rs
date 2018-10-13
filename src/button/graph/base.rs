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
use std::slice;
use std::collections::HashMap;

use holyhashmap::{self, HolyHashMap};

use super::traits::{
    Algo, Edges, GraphBase, Neighbors, NodeIndexable, NodeIndex, Nodes, Visitable,
};

pub trait NodeTrait: Ord + Hash {}
impl<N> NodeTrait for N where N: Ord + Hash {}

#[derive(
    Serialize,
    Deserialize,
    Default,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
)]
pub struct NodeNeighbors {
    pub incoming: Vec<NodeIndex>,
    pub outgoing: Vec<NodeIndex>,
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
    nodes: HolyHashMap<N, NodeNeighbors>,
    edges: HolyHashMap<(NodeIndex, NodeIndex), E>,
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
    fn from_index(&'a self, index: NodeIndex) -> &'a Self::Node {
        self.nodes.from_index(index).unwrap().0
    }

    fn to_index(&self, node: &Self::Node) -> Option<NodeIndex> {
        self.nodes.to_index(node)
    }
}

impl<'a, N, E> Nodes<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
{
    type Iter = holyhashmap::Indices<'a, N, NodeNeighbors>;

    fn nodes(&'a self) -> Self::Iter {
        self.nodes.indices()
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

    fn incoming(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            iter: self.nodes.from_index(node).unwrap().1.incoming.iter(),
        }
    }

    fn outgoing(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            iter: self.nodes.from_index(node).unwrap().1.outgoing.iter(),
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
    type Map = HashMap<NodeIndex, T>;

    fn visit_map(&self) -> Self::Map {
        HashMap::with_capacity(self.node_count())
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
            nodes: HolyHashMap::with_capacity(nodes),
            edges: HolyHashMap::with_capacity(edges),
        }
    }

    /// Add node `n` to the graph. Returns the index of the node.
    pub fn add_node(&mut self, n: N) -> NodeIndex {
        let entry = self.nodes.entry(n);
        let index = entry.index();
        entry.or_default();
        index
    }

    /// Adds an edge to the graph. Returns the old weight of the edge if it
    /// already existed.
    pub fn add_edge(&mut self, a: NodeIndex, b: NodeIndex, weight: E) -> Option<E> {
        let old = self.edges.insert((a, b), weight);
        if old.is_some() {
            old
        } else {
            // New edge. It needs to be inserted into the node
            if let Some((_, v)) = self.nodes.from_index_mut(a) {
                v.outgoing.push(b);
            }

            if a != b {
                if let Some((_, v)) = self.nodes.from_index_mut(b) {
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
        index: NodeIndex,
        other: &Graph<N, E>,
    ) -> Option<NodeIndex> {
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
    iter: holyhashmap::Iter<'a, (NodeIndex, NodeIndex), E>,
}

impl<'a, E> Iterator for EdgesIter<'a, E>
where
    E: 'a,
{
    type Item = (NodeIndex, NodeIndex, &'a E);

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
    iter: slice::Iter<'a, NodeIndex>,
}

impl<'a> Iterator for NeighborsIter<'a> {
    type Item = NodeIndex;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke() {
        let mut g = Graph::new();
        let a = g.add_node("a");
        let b = g.add_node("b");
        assert_eq!(a, 0.into());
        assert_eq!(b, 1.into());
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

        let sccs = graph.tarjan_scc();
        assert_eq!(sccs.len(), 4);

        assert_eq!(sccs[0], vec![1.into(), 4.into(), 0.into()]);
        assert_eq!(sccs[1], vec![3.into(), 2.into()]);
        assert_eq!(sccs[2], vec![6.into(), 5.into()]);
        assert_eq!(sccs[3], vec![7.into()]);
    }
}
