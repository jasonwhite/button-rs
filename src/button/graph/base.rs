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
use std::collections::HashMap;
use std::hash::Hash;
use std::slice;

use holyhashmap::{self, HolyHashMap};

use super::index::{EdgeIndex, NodeIndex};
use super::traits::{
    Algo, Edges, GraphBase, Indexable, Neighbors, Nodes, Visitable,
};

pub trait NodeTrait: Eq + Hash {}
impl<N> NodeTrait for N where N: Eq + Hash {}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq, Hash)]
struct NodeNeighbors {
    /// Incoming edges. We store the index of the edge such that we can more
    /// easily access the edge data. It also simplifies the subgraph
    /// implementation.
    incoming: Vec<(NodeIndex, EdgeIndex)>,

    /// Outgoing edges. We store the index of the edge such that we can more
    /// easily access the edge data. It also simplifies the subgraph
    /// implementation.
    outgoing: Vec<(NodeIndex, EdgeIndex)>,
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

    fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl<'a, N, E> Indexable<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
{
    fn node_from_index(&'a self, index: NodeIndex) -> &'a Self::Node {
        self.nodes.from_index(index.into()).unwrap().0
    }

    fn node_to_index(&self, node: &Self::Node) -> Option<NodeIndex> {
        self.nodes.to_index(node).map(NodeIndex::from)
    }

    fn edge_from_index(
        &'a self,
        index: EdgeIndex,
    ) -> ((NodeIndex, NodeIndex), &'a Self::Edge) {
        let (edge, weight) = self.edges.from_index(index.into()).unwrap();
        (*edge, weight)
    }

    fn edge_to_index(
        &self,
        edge: &(NodeIndex, NodeIndex),
    ) -> Option<EdgeIndex> {
        self.edges.to_index(edge).map(EdgeIndex::from)
    }
}

impl<'a, N, E> Nodes<'a> for Graph<N, E>
where
    N: NodeTrait + 'a,
{
    type Iter = NodesIter<'a, N>;

    fn nodes(&'a self) -> Self::Iter {
        NodesIter {
            iter: self.nodes.indices(),
        }
    }
}

pub struct NodesIter<'a, N: 'a> {
    iter: holyhashmap::Indices<'a, N, NodeNeighbors>,
}

impl<'a, N> Iterator for NodesIter<'a, N> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(NodeIndex::from)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(NodeIndex::from)
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last().map(NodeIndex::from)
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
            iter: self.edges.indices(),
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
            iter: self
                .nodes
                .from_index(node.into())
                .unwrap()
                .1
                .incoming
                .iter(),
        }
    }

    fn outgoing(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            iter: self
                .nodes
                .from_index(node.into())
                .unwrap()
                .1
                .outgoing
                .iter(),
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
        index.into()
    }

    /// Adds an edge to the graph. Returns the index of the edge.
    pub fn add_edge(
        &mut self,
        a: NodeIndex,
        b: NodeIndex,
        weight: E,
    ) -> EdgeIndex {
        let (edge, old) = self.edges.insert_full((a, b), weight);

        let edge: EdgeIndex = edge.into();

        if old.is_none() {
            // New edge. It needs to be inserted into the node
            if let Some((_, v)) = self.nodes.from_index_mut(a.into()) {
                v.outgoing.push((b, edge));
            }

            if a != b {
                if let Some((_, v)) = self.nodes.from_index_mut(b.into()) {
                    v.incoming.push((a, edge));
                }
            }
        }

        edge
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
        other.node_to_index(self.node_from_index(index))
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
    iter: holyhashmap::Indices<'a, (NodeIndex, NodeIndex), E>,
}

impl<'a, E> Iterator for EdgesIter<'a, E>
where
    E: 'a,
{
    type Item = EdgeIndex;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(EdgeIndex::from)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.iter.count()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(EdgeIndex::from)
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.iter.last().map(EdgeIndex::from)
    }
}

pub struct NeighborsIter<'a> {
    iter: slice::Iter<'a, (NodeIndex, EdgeIndex)>,
}

impl<'a> Iterator for NeighborsIter<'a> {
    type Item = (NodeIndex, EdgeIndex);

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

        g.add_edge(a, b, 42);
        g.add_edge(a, b, 1);

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

        let sccs = graph.tarjan_scc();
        assert_eq!(sccs.len(), 4);

        assert_eq!(sccs[0], vec![1.into(), 4.into(), 0.into()]);
        assert_eq!(sccs[1], vec![3.into(), 2.into()]);
        assert_eq!(sccs[2], vec![6.into(), 5.into()]);
        assert_eq!(sccs[3], vec![7.into()]);
    }
}
