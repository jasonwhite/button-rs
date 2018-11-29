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
use std::fs;
use std::io;
use std::path::Path;

use build_graph::BuildGraph;
use error::BuildError;
use graph::{Algo, Diff, Indexable, NodeIndex};
use res::ResourceState;

use bincode;
use tempfile::NamedTempFile;

/// The state of the build.
#[derive(Serialize, Deserialize, Default)]
pub struct BuildState {
    /// The build graph.
    pub graph: BuildGraph,

    /// A persistent queue of node indices that should be visited. Duplicate
    /// nodes don't matter here since nodes are only ever visited once when
    /// traversing the graph.
    pub queue: Vec<NodeIndex>,

    /// Resource state. This is used to detect changes to resources. If the
    /// resource does not exist in this map, then we don't yet know anything
    /// about this resource and it should not be considered "owned" by the
    /// build system. That is, the build system should never delete it if
    /// it doesn't "own" it.
    pub checksums: HashMap<NodeIndex, ResourceState>,
}

impl BuildState {
    /// Constructs a state from a new build graph. Used when an existing state
    /// does not exist on disk.
    pub fn from_graph(graph: BuildGraph) -> BuildState {
        // Everything needs to get built, so add all root nodes to the queue.
        let queue = graph.root_nodes().collect();

        BuildState {
            graph,
            queue,
            checksums: HashMap::new(),
        }
    }

    /// Reads the state from a file.
    pub fn from_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<BuildState, BuildError> {
        let f = fs::File::open(path)?;
        Ok(Self::from_reader(io::BufReader::new(f))?)
    }

    /// Reads the state from a stream.
    pub fn from_reader<R: io::Read>(
        mut reader: R,
    ) -> Result<BuildState, BuildError> {
        // Read the version string.
        let version: String = bincode::deserialize_from(&mut reader)?;

        if version != env!("CARGO_PKG_VERSION") {
            // Create a new build state when the version is different. This will
            // force a full rebuild when `update()` is called.
            Ok(BuildState::default())
        } else {
            let state = bincode::deserialize_from(reader)?;
            Ok(state)
        }
    }

    /// Writes the state to a stream.
    pub fn write_to<W: io::Write>(
        &self,
        mut writer: W,
    ) -> Result<(), BuildError> {
        bincode::serialize_into(&mut writer, env!("CARGO_PKG_VERSION"))?;
        bincode::serialize_into(writer, &self)?;
        Ok(())
    }

    /// Writes the state to a file. The file is atomically updated using
    /// a temporary file.
    pub fn write_to_path<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), BuildError> {
        let path = path.as_ref();

        let dir = path.parent().unwrap_or_else(|| Path::new("."));

        let mut tempfile = NamedTempFile::new_in(dir)?;

        self.write_to(io::BufWriter::new(&mut tempfile))?;

        tempfile.persist(path)?;

        Ok(())
    }

    /// Performs a non-destructive (no file system changes) synchronization of
    /// the state based on the new explicit graph.
    ///
    /// Deletion of removed output resources should be done before this.
    pub fn update(
        &mut self,
        graph: &BuildGraph,
        diff: &Diff,
    ) -> Result<(), BuildError> {
        // Remove edges before removing nodes so that the node removal has less
        // work to do. (If a node has fewer neighbors, it has fewer edges to
        // remove.)
        for index in diff.left_only_edges.iter() {
            assert!(self.graph.remove_edge(index).is_some());
        }

        // Remove nodes from the graph. This may invalidate the queue if the
        // queue contains any of the nodes being removed here. Thus, we need to
        // fix the queue after this removal.
        for index in diff.left_only_nodes.iter() {
            assert!(self.graph.remove_node(index).is_some());

            // Fix the checksums.
            self.checksums.remove(&index);
        }

        // Rebuild the queue with invalid indices filtered out.
        let mut queue: Vec<_> = self
            .queue
            .iter()
            .cloned()
            .filter(|&index| self.graph.contains_node_index(index))
            .collect();

        for index in diff.right_only_nodes.iter() {
            // New nodes should always be added to the queue such that they get
            // traversed.
            let node = graph.node_from_index(index);
            let index = self.graph.add_node(node.clone());
            queue.push(index);
        }

        for index in diff.right_only_edges.iter() {
            let ((a, b), weight) = graph.edge_from_index(index);

            // unwrapping because these nodes are guaranteed to exist in the
            // graph at this point already.
            let a = self.graph.node_to_index(graph.node_from_index(a)).unwrap();
            let b = self.graph.node_to_index(graph.node_from_index(b)).unwrap();

            self.graph.add_edge(a, b, *weight);
        }

        self.queue = queue;

        Ok(())
    }
}
