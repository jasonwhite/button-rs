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
use std::collections::{hash_set, HashMap, HashSet};
use std::iter;

use super::visit::{
    Algo, GraphBase, Neighbors, NodeIndexable, Nodes, Visitable,
};

/// A graph with a subset of nodes of the parent graph.
pub struct Subgraph<'a, G>
where
    G: 'a,
{
    parent: &'a G,
    nodes: HashSet<usize>,
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
    fn from_index(&'a self, index: usize) -> &'a Self::Node {
        assert!(self.nodes.contains(&index));
        self.parent.from_index(index)
    }

    fn to_index(&self, node: &Self::Node) -> Option<usize> {
        if let Some(index) = self.parent.to_index(node) {
            if self.nodes.contains(&index) {
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
    type Iter = iter::Cloned<hash_set::Iter<'a, usize>>;

    fn nodes(&'a self) -> Self::Iter {
        self.nodes.iter().cloned()
    }
}

impl<'a, G> Neighbors<'a> for Subgraph<'a, G>
where
    G: Neighbors<'a>,
{
    type Neighbors = NeighborsIter<'a, G>;

    fn incoming(&'a self, node: usize) -> Self::Neighbors {
        NeighborsIter {
            nodes: &self.nodes,
            iter: self.parent.incoming(node),
        }
    }

    fn outgoing(&'a self, node: usize) -> Self::Neighbors {
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
    nodes: &'a HashSet<usize>,
    iter: G::Neighbors,
}

impl<'a, G> Iterator for NeighborsIter<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(i) = self.iter.next() {
            // Only include neighbors that are in the subgraph.
            if self.nodes.contains(&i) {
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
    type Map = HashMap<usize, T>;

    fn visit_map(&self) -> Self::Map {
        HashMap::with_capacity(self.node_count())
    }
}

impl<'a, G> Subgraph<'a, G> {
    /// Creates a new subgraph with the given set of nodes.
    pub fn new<I>(parent: &'a G, nodes: I) -> Self
    where
        I: Iterator<Item = usize>,
    {
        Subgraph {
            parent,
            nodes: nodes.collect(),
        }
    }
}
