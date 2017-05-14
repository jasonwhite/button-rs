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

use petgraph::graphmap;

use resources;
use tasks;
use rules::Rules;

#[derive(Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Edge {
    /// An explicit edge is one that is user-defined in the build description.
    /// That is, it is *explicitly* declared.
    Explicit,

    /// An implicit edge is one that is automatically determined after the task
    /// is executed. That is, it is *implicitly* discovered. Tasks, when
    /// executed, return resources that are read from or written to. The edges
    /// associated with these resources are then implicit. It is usually the
    /// case that, for every implicit edge, there is an equivalent explicit
    /// edge.
    Implicit,
}

/// A node in the graph.
#[derive(Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub enum Node<'a> {
    Resource(&'a resources::File),
    Task(&'a Vec<tasks::Command>),
}

/// The build graph.
pub type BuildGraph<'a> = graphmap::DiGraphMap<Node<'a>, Edge>;

/// Returns a graph from the given rules.
pub fn from_rules(rules: &Rules) -> BuildGraph {
    let mut graph = BuildGraph::new();

    for rule in rules.iter() {
        let task = graph.add_node(Node::Task(&rule.tasks));

        for r in &rule.inputs {
            let node = graph.add_node(Node::Resource(r));
            graph.add_edge(node, task, Edge::Explicit);
        }

        for r in &rule.outputs {
            let node = graph.add_node(Node::Resource(r));
            graph.add_edge(task, node, Edge::Explicit);
        }
    }

    graph
}
