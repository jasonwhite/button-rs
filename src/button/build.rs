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

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use build_graph::{BuildGraph, BuildGraphExt, Edge, FromRules, Node};
use detect::Detected;
use logger::{EventLogger, TaskLogger};
use res::{self, Resource, ResourceState};
use rules::Rules;
use state::BuildState;
use task::{self, Task};

use graph::{
    Algo, Edges, IndexSet, Indexable, Neighbors, NodeIndex, Nodes, Subgraph,
};

use error::{Error, ResultExt};

/// A build failure. Contains each of the node indexes that failed and the
/// associated error.
#[derive(Fail, Debug)]
pub struct BuildFailure {
    errors: Vec<(NodeIndex, Error)>,
}

impl BuildFailure {
    pub fn new(errors: Vec<(NodeIndex, Error)>) -> BuildFailure {
        BuildFailure { errors }
    }
}

impl fmt::Display for BuildFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.errors.len() == 1 {
            write!(f, "Build failed with {} error", self.errors.len())
        } else {
            write!(f, "Build failed with {} errors", self.errors.len())
        }
    }
}

struct BuildContext<'a> {
    root: &'a Path,
    dryrun: bool,
    checksums: Mutex<HashMap<NodeIndex, ResourceState>>,

    // Detected inputs/outputs during the build.
    detected: Mutex<Vec<(NodeIndex, Detected)>>,
}

fn delete_resources<L>(
    state: &BuildState,
    to_remove: &IndexSet<NodeIndex>,
    root: &Path,
    threads: usize,
    logger: &L,
    dryrun: bool,
) -> Result<(), Error>
where
    L: EventLogger,
{
    if to_remove.is_empty() {
        return Ok(());
    }

    let graph = &state.graph;
    let checksums = &state.checksums;

    graph
        .traverse(
            |tid, index, node| {
                if let Node::Resource(r) = node {
                    // Only delete the resource if its in our set of removed
                    // resources and if the state has been computed. A computed
                    // state indicates that the build system "owns" the
                    // resource.
                    if !graph.is_root_node(index)
                        && to_remove.contains(&index)
                        && checksums.contains_key(&index)
                    {
                        logger.delete(tid, r)?;

                        if !dryrun {
                            r.delete(root)?;
                        }
                    }
                }

                // Let the traversal proceed to the next node.
                Ok(true)
            },
            threads,
            true,
        )
        .map_err(BuildFailure::new)?; // TODO: Return a ResourceDeletion error.

    Ok(())
}

/// Updates the build state with the build graph loaded from the on-disk rules.
///
/// This is one of the most important algorithms in the build system.
fn sync_state<L>(
    state: &mut BuildState,
    graph: BuildGraph,
    root: &Path,
    threads: usize,
    logger: &L,
    dryrun: bool,
) -> Result<(), Error>
where
    L: EventLogger,
{
    // Diff with the explicit subgraph in order to have a one-to-one comparison
    // with the rules build graph.
    let diff = state.graph.explicit_subgraph().diff(&graph);

    let nodes_to_delete: Vec<_> = diff
        .left_only_edges
        .iter()
        .map(|index| {
            let (_, b) = state.graph.edge_from_index(index).0;
            b
        })
        .collect();

    let nodes_to_delete: IndexSet<_> = nodes_to_delete.into_iter().collect();

    // Delete the non-root resources in reverse-topological order that we own.
    delete_resources(state, &nodes_to_delete, root, threads, logger, dryrun)?;

    // Remove edges before removing nodes so that the node removal has less work
    // to do. (If a node has fewer neighbors, it has fewer edges to remove.)
    for index in diff.left_only_edges.iter() {
        assert!(state.graph.remove_edge(index).is_some());
    }

    // Remove nodes from the graph. This may invalidate the queue if the queue
    // contains any of the nodes being removed here. Thus, we need to fix the
    // queue after this removal.
    for index in diff.left_only_nodes.iter() {
        assert!(state.graph.remove_node(index).is_some());

        // Fix the checksums.
        state.checksums.remove(&index);
    }

    // Rebuild the queue with invalid indices filtered out.
    let mut queue: Vec<_> = state
        .queue
        .iter()
        .cloned()
        .filter(|&index| state.graph.contains_node_index(index))
        .collect();

    for index in diff.right_only_nodes.iter() {
        // New nodes should always be added to the queue such that they get
        // traversed.
        let node = graph.node_from_index(index);
        let index = state.graph.add_node(node.clone());
        queue.push(index);
    }

    for index in diff.right_only_edges.iter() {
        let ((a, b), weight) = graph.edge_from_index(index);

        // unwrapping because these nodes are guaranteed to exist in the graph
        // at this point already.
        let a = state.graph.node_to_index(graph.node_from_index(a)).unwrap();
        let b = state.graph.node_to_index(graph.node_from_index(b)).unwrap();

        state.graph.add_edge(a, b, *weight);
    }

    state.queue = queue;

    Ok(())
}

/// Updates the build graph with the detected inputs/outputs.
///
/// Note that there is one case where this can fail: adding a dependency on
/// a non-root node. Such a scenario can change the build order or create a race
/// condition.
fn sync_detected<L>(
    graph: &mut BuildGraph,
    detected: Vec<(NodeIndex, Detected)>,
    checksums: &mut HashMap<NodeIndex, ResourceState>,
    _root: &Path,
    _threads: usize,
    _logger: &L,
    _dryrun: bool,
) -> Result<(), Error> {
    for (node, detected) in detected {
        let mut inputs_to_remove = Vec::new();

        // Find edges that can be removed.
        for (index, edge) in graph.incoming(node) {
            if graph.edge_from_index(edge).1 == &Edge::Implicit {
                // We can safely assume this will always be a resource-type
                // node.
                let r = match graph.node_from_index(index) {
                    Node::Resource(r) => r,
                    _ => unreachable!(),
                };

                if !detected.inputs.contains(r) {
                    // This node is no longer being detected as an input. We
                    // need to remove it from the graph.
                    inputs_to_remove.push(index);
                }
            }
        }

        for input in inputs_to_remove {
            let edge_index = graph.edge_to_index(input, node).unwrap();
            graph.remove_edge(edge_index);

            // Remove the node if it has become disconnected from the graph.
            // Orphaned nodes shouldn't cause any problems, but cleaning them up
            // immediately after they form simplifies some logic and keeps the
            // graph looking clean.
            if graph.is_root_node(input) && graph.is_terminal_node(input) {
                graph.remove_node(input);
            }

            // Any time a resource is removed from the graph, it needs to be
            // removed from the checksums.
            checksums.remove(&input);
        }

        // Find new edges.
        for input in detected.inputs {
            let input = Node::Resource(input);

            if let Some(index) = graph.node_to_index(&input) {
                if !graph.contains_edge_by_index(index, node) {
                    // It's only valid to add an edge to this node if the node
                    // is a root node.
                    // TODO: Return an error if it's not a root node!
                    if graph.is_root_node(index) {
                        graph.add_edge(index, node, Edge::Implicit);
                    }
                }
            } else {
                // A new node! It's always valid to add a new node as an input.
                let index = graph.add_node(input);
                graph.add_edge(index, node, Edge::Implicit);
            }
        }

        // For detected outputs, we must only
        //  1. add an edge to new nodes.
        //  2. delete resources *after* the graph has been fully updated and in
        //     reverse topological order. That way, if anything fails, nothing
        //     has been deleted yet.
    }

    Ok(())
}

/// Iterator over nodes that should be traversed during the build.
///
/// Yields nodes that should be queued. Root resources are queued if they have
/// changed. The parent task of non-root resources are queued if they have
/// changed.
///
/// This does not modify the stored checksums. The checksums will be updated as
/// the graph is traversed so that it represents the most recent state at the
/// time of the build. There may be some time delay between this step and
/// actually starting the build.
///
/// Unfortunately, this also means that we are hashing every file *twice*. Once
/// before the build and once during the build.
///
/// In the future, there will be a daemon process continuously monitoring file
/// changes and maintaining a queue in the background alleviating this build
/// latency.
struct DirtyNodes<'a> {
    root: &'a Path,
    graph: &'a BuildGraph,
    nodes: <BuildGraph as Nodes<'a>>::Iter,
    checksums: &'a HashMap<NodeIndex, ResourceState>,
}

impl<'a> DirtyNodes<'a> {
    pub fn new(
        root: &'a Path,
        graph: &'a BuildGraph,
        checksums: &'a HashMap<NodeIndex, ResourceState>,
    ) -> DirtyNodes<'a> {
        DirtyNodes {
            root,
            graph,
            nodes: graph.nodes(),
            checksums,
        }
    }
}

impl<'a> Iterator for DirtyNodes<'a> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(index) = self.nodes.next() {
            if let Node::Resource(r) = self.graph.node_from_index(index) {
                match self.checksums.get(&index) {
                    Some(stored_state) => {
                        // Compute the current state and see if they differ.
                        if let Ok(current_state) = r.state(self.root) {
                            if stored_state != &current_state {
                                if let Some((parent, _)) =
                                    self.graph.incoming(index).next()
                                {
                                    // If this is a non-root node, return
                                    // the task that produces this resource
                                    // instead.
                                    return Some(parent);
                                } else {
                                    return Some(index);
                                }
                            }
                        } else if let Some((parent, _)) =
                            self.graph.incoming(index).next()
                        {
                            // If this is a non-root node, return
                            // the task that produces this resource
                            // instead.
                            return Some(parent);
                        } else {
                            return Some(index);
                        }
                    }
                    None => {
                        // Only queue if this is a root node and if the
                        // checksum has never been computed.
                        if self.graph.is_root_node(index) {
                            return Some(index);
                        }
                    }
                }
            }
        }

        None
    }
}

pub struct Build<'a> {
    /// Path to the root of the project. This is used to ensure tasks start in
    /// the correct working directory.
    root: &'a Path,

    /// Path to the build state. If this has a parent directory, the parent
    /// directory must exist.
    state: &'a Path,
}

impl<'a> Build<'a> {
    /// Creates a new `Build`.
    pub fn new(root: &'a Path, state: &'a Path) -> Build<'a> {
        Build { root, state }
    }

    /// Cleans all outputs of the build and the build state.
    ///
    /// This does *not* clean up build logs or anything else. Since the client
    /// is creating these things, it's up to the client to clean them up.
    pub fn clean<L>(
        &self,
        dryrun: bool,
        threads: usize,
        logger: &L,
    ) -> Result<(), Error>
    where
        L: EventLogger,
    {
        let state = match fs::File::open(self.state) {
            Ok(f) => BuildState::from_reader(io::BufReader::new(f))
                .with_context(|_| {
                    format!(
                        "Failed loading build state from file {:?}. \
                         Is it corrupted? Consider doing a `git clean -fdx` \
                         or equivalent.",
                        self.state
                    )
                })?,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    // Nothing to do if it doesn't exist.
                    return Ok(());
                } else {
                    // Some other fatal IO error occurred.
                    return Err(err.into());
                }
            }
        };

        // Delete resources in reverse topological order.
        state
            .graph
            .traverse(
                |tid, index, node| {
                    if let Node::Resource(r) = node {
                        // Only delete the resource if the state has been
                        // computed. A computed state indicates that the build
                        // system "owns" the resource.
                        if !state.graph.is_root_node(index)
                            && state.checksums.contains_key(&index)
                        {
                            logger.delete(tid, r)?;

                            if !dryrun {
                                r.delete(self.root)?;
                            }
                        }
                    }

                    // Let the traversal proceed to the next node.
                    Ok(true)
                },
                threads,
                true,
            )
            .map_err(BuildFailure::new)?;
        // TODO: Return a ResourceDeletion error.

        // Delete the build state
        fs::remove_file(self.state)?;

        Ok(())
    }

    /// Runs an incremental build.
    ///
    /// The build algorithm proceeds as follows:
    ///
    ///  1. Load the build state if possible. If there is no build state,
    ///     creates a new one.
    ///
    ///     (a) Updates the build state with the new build graph (which is
    ///         constructed from the passed in build rules). This is done
    ///         diffing the set of nodes in the two graphs.
    ///
    ///     (b) For resources that don't exist in the new graph, they are
    ///         deleted from disk. Resources are deleted in reverse topological
    ///         order such that files are deleted before their parent
    ///         directories. If any resources fail to be deleted, the
    ///         build fails. Resources that are not owned by the build system
    ///         yet (i.e., resources whose state has not yet been computed) are
    ///         not deleted.
    ///
    ///  2. Find out-of-date nodes and queue them. For root resources that have
    ///     changed state, queue them. For non-root resources that have changed,
    ///     queue the task that produces them.
    ///
    ///     If the queue is empty after this, then there is nothing to do.
    ///
    ///  3. Create a subgraph from the queued nodes.
    ///
    ///  4. Traverse the subgraph in topological order, thereby building
    ///     everything. For resources that don't change state after being built,
    ///     traversal doesn't go any further.
    ///
    ///  5. For any nodes that failed to build, add them to the queue for
    ///     execution next time. We don't want the build to succeed as long as
    ///     there are failing nodes.
    ///
    ///  6. Persist the build state to disk. This is done atomically using a
    ///     temporary file and rename.
    pub fn build<L>(
        &self,
        rules: Rules,
        dryrun: bool,
        threads: usize,
        logger: &mut L,
    ) -> Result<(), Error>
    where
        L: EventLogger,
    {
        logger.begin_build(threads)?;

        let result = self.build_impl(rules, dryrun, threads, logger);

        logger.end_build(&result)?;
        result
    }

    fn build_impl<L>(
        &self,
        rules: Rules,
        dryrun: bool,
        threads: usize,
        logger: &L,
    ) -> Result<(), Error>
    where
        L: EventLogger,
    {
        let graph = BuildGraph::from_rules(rules)
            .context("Failed to create build graph from rules")?;

        // Load/create the build state.
        let BuildState {
            mut graph,
            mut queue,
            checksums,
        } = {
            match fs::File::open(self.state) {
                Ok(f) => {
                    let mut state =
                        BuildState::from_reader(io::BufReader::new(f))
                            .with_context(|_| {
                                format!(
                                "Failed loading build state from file {:?}. \
                                 Is it corrupted? Consider doing a \
                                 `git clean -fdx` or equivalent.",
                                self.state
                            )
                            })?;

                    sync_state(
                        &mut state, graph, self.root, threads, logger, dryrun,
                    )
                    .context("Failed updating build graph")?;

                    state
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::NotFound {
                        // If it doesn't exist, create it.
                        BuildState::from_graph(graph)
                    } else {
                        // Some other fatal IO error occurred.
                        return Err(err.into());
                    }
                }
            }
        };

        queue.extend(DirtyNodes::new(self.root, &graph, &checksums));

        if queue.is_empty() {
            // Don't bother traversing the graph if the queue is empty.
            return Ok(());
        }

        let context = BuildContext {
            root: self.root,
            dryrun,
            checksums: Mutex::new(checksums),
            detected: Mutex::new(Vec::new()),
        };

        let result = {
            // Create the subgraph from the queued nodes.
            let subgraph = Subgraph::new(
                &graph,
                graph.dfs(queue.into_iter()),
                graph.edges(),
            );

            // Build the subgraph.
            subgraph.traverse(
                |tid, index, node| {
                    build_node(&context, tid, index, node, logger)
                },
                threads,
                false,
            )
        };

        let queue = {
            if let Err(errors) = &result {
                // Queue all failed tasks so that they get visited again next
                // time.
                errors.iter().map(|x| x.0).collect()
            } else {
                Vec::new()
            }
        };

        let BuildContext {
            root: _,
            dryrun: _,
            checksums,
            detected,
        } = context;
        let mut checksums = checksums.into_inner().unwrap();
        let detected = detected.into_inner().unwrap();

        // TODO: Add the detected inputs/outputs to the build graph. We must not
        // modify the build order when adding new edges to the graph. That is,
        // we can only add edges to *root* nodes. If we attempt to do otherwise,
        // then the build state shouldn't be committed.
        sync_detected(
            &mut graph, detected, &mut checksums, self.root, threads, logger,
            dryrun,
        )?;

        // Serialize the state. This must be the last thing that we do. If
        // anything fails above (e.g., failing to delete a resource), the state
        // will remain untouched and the error should be reproducible. Note that
        // task failures should not prevent the state from being saved. Instead,
        // those are added to the queue to be executed again.
        BuildState {
            graph,
            queue,
            checksums,
        }
        .write_to_path(self.state)
        .with_context(|_| {
            format!("Failed writing build state to {:?}", self.state)
        })?;

        result.map_err(BuildFailure::new)?;

        Ok(())
    }
}

fn build_node<L>(
    context: &BuildContext,
    tid: usize,
    index: NodeIndex,
    node: &Node,
    logger: &L,
) -> Result<bool, Error>
where
    L: EventLogger,
{
    match node {
        Node::Resource(r) => build_resource(context, tid, index, r),
        Node::Task(t) => build_task(context, tid, index, t, logger),
    }
}

fn build_resource(
    context: &BuildContext,
    _tid: usize,
    index: NodeIndex,
    node: &res::Any,
) -> Result<bool, Error> {
    let state = node.state(context.root)?;

    let mut checksums = context.checksums.lock().unwrap();

    let ret = if let Some(prev_state) = checksums.get(&index) {
        // Only need to proceed down the graph if this resource changed.
        Ok(&state != prev_state)
    } else {
        // Previous state wasn't computed. Unconditionally proceed down the
        // graph.
        Ok(true)
    };

    checksums.insert(index, state);

    ret
}

fn build_task<L>(
    context: &BuildContext,
    tid: usize,
    index: NodeIndex,
    node: &task::List,
    logger: &L,
) -> Result<bool, Error>
where
    L: EventLogger,
{
    for task in node.iter() {
        let mut task_logger = logger.start_task(tid, &task)?;

        if context.dryrun {
            task_logger.finish(&Ok(Detected::new()))?;
        } else {
            let result = task.execute(context.root, &mut task_logger);

            task_logger.finish(&result)?;

            // Accumulate the detected inputs/outputs such that we can add them
            // to the implicit resources to the graph later. (We cannot modify
            // the build graph while traversing it.)
            context.detected.lock().unwrap().push((index, result?));
        }
    }

    Ok(true)
}
