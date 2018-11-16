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
use super::traits::{
    Algo, Edges, GraphBase, Indexable, Neighbors, Nodes, Visitable,
};

use super::index::{EdgeIndex, IndexSet, IndexSetIter, NodeIndex};

/// A graph with a subset of nodes and edges.
pub struct Subgraph<'a, G>
where
    G: 'a,
{
    graph: &'a G,

    // Nodes that are in the graph.
    nodes: IndexSet<NodeIndex>,

    // Edges that are in the graph.
    edges: IndexSet<EdgeIndex>,
}

impl<'a, G> Subgraph<'a, G> {
    /// Creates a new subgraph with the given set of nodes.
    pub fn new<I, J>(graph: &'a G, nodes: I, edges: J) -> Self
    where
        I: Iterator<Item = NodeIndex>,
        J: Iterator<Item = EdgeIndex>,
    {
        Subgraph {
            graph,
            nodes: nodes.collect(),
            edges: edges.collect(),
        }
    }
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

    fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl<'a, G> Indexable<'a> for Subgraph<'a, G>
where
    G: Indexable<'a>,
{
    fn node_from_index(&'a self, index: NodeIndex) -> &'a Self::Node {
        assert!(
            self.nodes.contains(&index),
            "subgraph does not contain node"
        );
        self.graph.node_from_index(index)
    }

    fn node_to_index(&self, node: &Self::Node) -> Option<NodeIndex> {
        if let Some(index) = self.graph.node_to_index(node) {
            if self.nodes.contains(&index) {
                return Some(index);
            }
        }

        None
    }

    fn edge_from_index(
        &'a self,
        index: EdgeIndex,
    ) -> ((NodeIndex, NodeIndex), &'a Self::Edge) {
        assert!(
            self.edges.contains(&index),
            "subgraph does not contain edge"
        );
        self.graph.edge_from_index(index)
    }

    fn edge_to_index(
        &self,
        edge: &(NodeIndex, NodeIndex),
    ) -> Option<EdgeIndex> {
        if let Some(index) = self.graph.edge_to_index(edge) {
            if self.edges.contains(&index) {
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
    type Iter = IndexSetIter<'a, NodeIndex>;

    fn nodes(&'a self) -> Self::Iter {
        self.nodes.iter()
    }
}

impl<'a, G> Edges<'a> for Subgraph<'a, G>
where
    G: GraphBase + 'a,
{
    type Iter = IndexSetIter<'a, EdgeIndex>;

    fn edges(&'a self) -> Self::Iter {
        self.edges.iter()
    }
}

impl<'a, G> Neighbors<'a> for Subgraph<'a, G>
where
    G: Neighbors<'a>,
{
    type Neighbors = NeighborsIter<'a, G>;

    fn incoming(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            iter: self.graph.incoming(node),
            nodes: &self.nodes,
            edges: &self.edges,
        }
    }

    fn outgoing(&'a self, node: NodeIndex) -> Self::Neighbors {
        NeighborsIter {
            iter: self.graph.outgoing(node),
            nodes: &self.nodes,
            edges: &self.edges,
        }
    }
}

pub struct NeighborsIter<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    iter: G::Neighbors,
    nodes: &'a IndexSet<NodeIndex>,
    edges: &'a IndexSet<EdgeIndex>,
}

impl<'a, G> Iterator for NeighborsIter<'a, G>
where
    G: Neighbors<'a> + 'a,
{
    type Item = (NodeIndex, EdgeIndex);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, edge)) = self.iter.next() {
            // Only include neighbors that are in the subgraph and only include
            // edges to neighbors that are in the subgraph.
            if self.nodes.contains(&node) && self.edges.contains(&edge) {
                return Some((node, edge));
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
