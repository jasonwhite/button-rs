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
use bit_set::{self, BitSet};

use super::traits::{
    Algo, GraphBase, Neighbors, NodeIndex, NodeIndexable, Nodes, Visitable,
};

/// A graph with a subset of nodes of the parent graph.
pub struct Subgraph<'a, G>
where
    G: 'a,
{
    parent: &'a G,
    nodes: BitSet,
}

impl<'a, G> GraphBase for Subgraph<'a, G>
where
    G: GraphBase,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl<'a, G> NodeIndexable<'a> for Subgraph<'a, G>
where
    G: NodeIndexable<'a>,
{
    fn from_index(&'a self, index: NodeIndex) -> &'a Self::Node {
        debug_assert!(self.nodes.contains(index.into()));
        self.parent.from_index(index)
    }

    fn to_index(&self, node: &Self::Node) -> Option<NodeIndex> {
        if let Some(index) = self.parent.to_index(node) {
            if self.nodes.contains(index.into()) {
                return Some(index);
            }
        }

        None
    }
}

impl<'a, G> Nodes<'a> for Subgraph<'a, G>
where
    G: GraphBase + 'a,
{
    type Iter = Iter<'a>;

    fn nodes(&'a self) -> Self::Iter {
        Iter {
            iter: self.nodes.iter(),
        }
    }
}

pub struct Iter<'a> {
    iter: bit_set::Iter<'a, u32>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(NodeIndex::from)
    }
}

impl<'a, G> Neighbors<'a> for Subgraph<'a, G>
where
    G: Neighbors<'a>,
{
    type Neighbors = NeighborsIter<'a, G>;

    fn incoming(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            nodes: &self.nodes,
            iter: self.parent.incoming(node),
        }
    }

    fn outgoing(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            nodes: &self.nodes,
            iter: self.parent.outgoing(node),
        }
    }
}

pub struct NeighborsIter<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    nodes: &'a BitSet,
    iter: G::Neighbors,
}

impl<'a, G> Iterator for NeighborsIter<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.iter.next() {
            // Only include neighbors that are in the subgraph.
            if self.nodes.contains(i.into()) {
                return Some(i);
            }
        }

        None
    }
}

/// Be able to use algorithms with this graph.
impl<'a, G> Algo<'a> for Subgraph<'a, G> where G: Neighbors<'a> + 'a {}

impl<'a, G, T> Visitable<T> for Subgraph<'a, G>
where
    Self: GraphBase,
{
    /// We have to use a HashMap for the visit map because the node indices may
    /// be sparse.
    type Map = Vec<Option<T>>;

    fn visit_map(&self) -> Self::Map {
        Vec::with_capacity(self.node_count())
    }
}

impl<'a, G> Subgraph<'a, G> {
    /// Creates a new subgraph with the given set of nodes.
    pub fn new<I>(parent: &'a G, nodes: I) -> Self
    where
        I: Iterator<Item = NodeIndex>,
    {
        Subgraph {
            parent,
            nodes: nodes.map(NodeIndex::into).collect(),
        }
    }
}
