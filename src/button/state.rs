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
use std::mem;
use std::path::Path;

use build_graph::BuildGraph;
use error::Error;
use graph::{Algo, NodeIndex, Indexable, Nodes};
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<BuildState, Error> {
        let f = fs::File::open(path)?;
        Ok(Self::from_reader(io::BufReader::new(f))?)
    }

    /// Reads the state from a stream.
    pub fn from_reader<R: io::Read>(
        mut reader: R,
    ) -> Result<BuildState, bincode::Error> {
        // Read the version string.
        let version: String = bincode::deserialize_from(&mut reader)?;

        if version != env!("CARGO_PKG_VERSION") {
            // Create a new build state when the version is different. This will
            // force a full rebuild when `update()` is called.
            Ok(BuildState::default())
        } else {
            bincode::deserialize_from(reader)
        }
    }

    /// Writes the state to a stream.
    pub fn write_to<W: io::Write>(
        &self,
        mut writer: W,
    ) -> Result<(), bincode::Error> {
        bincode::serialize_into(&mut writer, env!("CARGO_PKG_VERSION"))?;
        bincode::serialize_into(writer, &self)
    }

    /// Writes the state to a file. The file is atomically updated using
    /// a temporary file.
    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let path = path.as_ref();

        let dir = path.parent().unwrap_or_else(|| Path::new("."));

        let mut tempfile = NamedTempFile::new_in(dir)?;

        self.write_to(io::BufWriter::new(&mut tempfile))?;

        tempfile.persist(path)?;

        Ok(())
    }

    /// Updates the build state with the given build graph.
    ///
    /// Returns the old build state and the list of non-root nodes that have
    /// been removed from the graph. This information can be used to delete
    /// resources in reverse topological order.
    pub fn update(
        &mut self,
        graph: BuildGraph,
    ) -> (BuildState, Vec<NodeIndex>) {
        let mut removed = Vec::new();

        // Fix the indices in the queue.
        let mut queue: Vec<_> = self
            .queue
            .iter()
            .filter_map(|i| self.graph.translate_index(*i, &graph))
            .collect();

        // Find removed output nodes.
        for index in self.graph.non_root_nodes() {
            if !graph.contains_node(self.graph.node_from_index(index)) {
                removed.push(index);
            }
        }

        // Add new nodes to the queue.
        for i in graph.nodes() {
            let node = graph.node_from_index(i);
            if !self.graph.contains_node(node) {
                if let Some(index) = graph.node_to_index(node) {
                    queue.push(index);
                }
            }
        }

        // Fix the indices in the checksums.
        let mut checksums = HashMap::new();
        for (i, checksum) in &self.checksums {
            if let Some(i) = self.graph.translate_index(*i, &graph) {
                checksums.insert(i, checksum.clone());
            }
        }

        (
            mem::replace(
                self,
                BuildState {
                    graph,
                    queue,
                    checksums,
                },
            ),
            removed,
        )
    }
}
