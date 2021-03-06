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

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use crate::build_graph::{BuildGraph, BuildGraphExt, Edge, FromRules, Node};
use crate::detect::Detected;
use crate::error::{
    BuildError, Error, ErrorKind, Fail, InvalidEdges, ResultExt,
};
use crate::events::{EventSender, EventSink};
use crate::graph::{
    Algo, Edges, IndexSet, Indexable, Neighbors, NodeIndex, Nodes, Subgraph,
};
use crate::res::{self, Resource, ResourceState};
use crate::rules::Rules;
use crate::state::BuildState;
use crate::task::{self, Task};

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    graph: &'a BuildGraph,
    checksums: Mutex<HashMap<NodeIndex, ResourceState>>,

    // Detected inputs/outputs during the build.
    detected: Mutex<Vec<(NodeIndex, Detected)>>,
}

fn delete_resources(
    state: &BuildState,
    to_remove: &IndexSet<NodeIndex>,
    root: &Path,
    threads: usize,
    events: EventSender,
    dryrun: bool,
) -> Result<(), BuildError> {
    if to_remove.is_empty() {
        return Ok(());
    }

    let graph = &state.graph;
    let checksums = &state.checksums;

    graph
        .traverse(
            |tid, index, node, events| {
                if let Node::Resource(r) = node {
                    // Only delete the resource if its in our set of removed
                    // resources and if the state has been computed. A computed
                    // state indicates that the build system "owns" the
                    // resource.
                    if !graph.is_root_node(index)
                        && to_remove.contains(&index)
                        && checksums.contains_key(&index)
                    {
                        let result =
                            if dryrun { Ok(()) } else { r.delete(root) };

                        events.delete(tid, r.clone(), &result);

                        result?;
                    }
                }

                // Let the traversal proceed to the next node.
                Ok(true)
            },
            &IndexSet::new(),
            threads,
            true,
            events,
        )
        .map_err(ErrorKind::DeleteErrors)?;

    Ok(())
}

/// Updates the build state with the build graph loaded from the on-disk rules.
fn sync_state(
    state: &mut BuildState,
    graph: &BuildGraph,
    root: &Path,
    threads: usize,
    events: EventSender,
    dryrun: bool,
) -> Result<(), BuildError> {
    // Diff with the explicit subgraph in order to have a one-to-one comparison
    // with the rules build graph.
    let diff = state.graph.explicit_subgraph().diff(graph);

    let nodes_to_delete: Vec<_> = diff
        .left_only_edges
        .iter()
        .map(|index| {
            let (_, b) = state.graph.edge_from_index(index).0;
            b
        })
        .collect();

    let nodes_to_delete: IndexSet<_> = nodes_to_delete.into_iter().collect();

    // Delete the non-root resources that we own in reverse-topological order.
    delete_resources(state, &nodes_to_delete, root, threads, events, dryrun)?;

    // Non-destructive sync of the state's data structures.
    state.update(graph, &diff);

    Ok(())
}

fn removed_implicit_inputs(
    graph: &mut BuildGraph,
    node: NodeIndex,
    inputs: &HashSet<res::Any>,
) -> Vec<NodeIndex> {
    let mut inputs_to_remove = Vec::new();

    // Find edges that can be removed.
    for (index, edge) in graph.incoming(node) {
        if graph.edge_from_index(edge).1 == &Edge::Implicit {
            // We can safely assume this will always be a resource-type node.
            let r = graph.node_from_index(index).as_res();

            if !inputs.contains(r) {
                // This node is no longer being detected as an input. We need to
                // remove it from the graph.
                inputs_to_remove.push(index);
            }
        }
    }

    inputs_to_remove
}

fn sync_removed_inputs(
    graph: &mut BuildGraph,
    node: NodeIndex,
    inputs: &HashSet<res::Any>,
    checksums: &mut HashMap<NodeIndex, ResourceState>,
) {
    for input in removed_implicit_inputs(graph, node, inputs) {
        let edge_index = graph.edge_to_index(input, node).unwrap();
        graph.remove_edge(edge_index);

        // Remove the node if it has become disconnected from the graph.
        // Orphaned nodes shouldn't cause any problems, but cleaning them up
        // immediately after they form simplifies some logic and keeps the graph
        // looking clean.
        if graph.is_root_node(input) && graph.is_terminal_node(input) {
            graph.remove_node(input);
        }

        // Any time a resource is removed from the graph, it needs to be removed
        // from the checksums.
        checksums.remove(&input);
    }
}

fn sync_added_inputs(
    graph: &mut BuildGraph,
    node: NodeIndex,
    inputs: HashSet<res::Any>,
    checksums: &mut HashMap<NodeIndex, ResourceState>,
    root: &Path,
) {
    for input in inputs {
        let input = Node::Resource(input);

        if let Some(index) = graph.node_to_index(&input) {
            if !graph.contains_edge_by_index(index, node) {
                // It's only valid to add an edge to this node if the node
                // is a root node.
                if graph.is_root_node(index) {
                    graph.add_edge(index, node, Edge::Implicit);
                } else {
                    // Adding this edge to the graph would have caused the build
                    // order to change.
                    //
                    // It should not be possible to reach this. The task should
                    // have failed before reaching this spot.
                    unreachable!();
                }
            }
        } else {
            // Calculate the checksum so the build doesn't see this as
            // changed next time.
            let checksum = input.as_res().state(root);

            // A new node! It's always valid to add a new node as an input.
            let index = graph.add_node(input);
            graph.add_edge(index, node, Edge::Implicit);

            if let Ok(checksum) = checksum {
                assert!(checksums.insert(index, checksum).is_none());
            }
        }
    }
}

/// Updates the build graph with the detected inputs/outputs.
///
/// Note that there is one case where this can fail: adding a dependency on
/// a non-root node. Such a scenario can change the build order or create a race
/// condition.
fn sync_detected(
    graph: &mut BuildGraph,
    detected: Vec<(NodeIndex, Detected)>,
    checksums: &mut HashMap<NodeIndex, ResourceState>,
    root: &Path,
    _threads: usize,
    _dryrun: bool,
) -> Result<(), BuildError> {
    for (node, Detected { inputs, .. }) in detected {
        // Sync inputs
        sync_removed_inputs(graph, node, &inputs, checksums);
        sync_added_inputs(graph, node, inputs, checksums, root);

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

    /// Number of threads to use for the build.
    threads: usize,

    /// Channel for sending events to the event thread.
    event_sender: EventSender,
}

impl<'a> Build<'a> {
    /// Creates a new `Build`.
    pub fn new(
        root: &'a Path,
        state: &'a Path,
        threads: usize,
        event_sender: EventSender,
    ) -> Build<'a> {
        Build {
            root,
            state,
            threads,
            event_sender,
        }
    }

    /// Cleans all outputs of the build and the build state.
    ///
    /// This does *not* clean up build logs or anything else. Since the client
    /// is creating these things, it's up to the client to clean them up.
    pub fn clean(&self, dryrun: bool) -> Result<(), BuildError> {
        self.event_sender.begin_build(self.threads, "clean");

        let result = self.clean_impl(dryrun);

        self.event_sender.end_build(&result);
        result
    }

    pub fn clean_impl(&self, dryrun: bool) -> Result<(), BuildError> {
        let state = match fs::File::open(self.state) {
            Ok(f) => BuildState::from_reader(io::BufReader::new(f))
                .with_context(|_| {
                    ErrorKind::LoadState(self.state.to_path_buf())
                })?,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    // Nothing to do if it doesn't exist.
                    return Ok(());
                } else {
                    // Some other fatal IO error occurred.
                    return Err(err
                        .context(ErrorKind::LoadState(self.state.to_path_buf()))
                        .into());
                }
            }
        };

        let root = self.root;

        // Delete resources in reverse topological order.
        state
            .graph
            .traverse(
                |tid, index, node, events| {
                    if let Node::Resource(r) = node {
                        // Only delete the resource if the state has been
                        // computed. A computed state indicates that the build
                        // system "owns" the resource.
                        if !state.graph.is_root_node(index)
                            && state.checksums.contains_key(&index)
                        {
                            let result =
                                if dryrun { Ok(()) } else { r.delete(root) };

                            events.delete(tid, r.clone(), &result);

                            result?;
                        }
                    }

                    // Let the traversal proceed to the next node.
                    Ok(true)
                },
                &IndexSet::new(),
                self.threads,
                true,
                self.event_sender.clone(),
            )
            .map_err(ErrorKind::DeleteErrors)?;

        // Delete the build state
        fs::remove_file(self.state).with_context(|_| {
            ErrorKind::CleanState(self.state.to_path_buf())
        })?;

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
    pub fn build(&self, rules: Rules, dryrun: bool) -> Result<(), BuildError> {
        self.event_sender.begin_build(self.threads, "build");

        let result = self.build_impl(rules, dryrun);

        self.event_sender.end_build(&result);
        result
    }

    fn build_impl(&self, rules: Rules, dryrun: bool) -> Result<(), BuildError> {
        let graph =
            BuildGraph::from_rules(rules).context(ErrorKind::BuildGraph)?;

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
                                ErrorKind::LoadState(self.state.to_path_buf())
                            })?;

                    sync_state(
                        &mut state,
                        &graph,
                        self.root,
                        self.threads,
                        self.event_sender.clone(),
                        dryrun,
                    )
                    .context(ErrorKind::SyncState)?;

                    state
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::NotFound {
                        // If it doesn't exist, create it.
                        BuildState::from_graph(graph)
                    } else {
                        // Some other fatal IO error occurred.
                        return Err(err
                            .context(ErrorKind::LoadState(
                                self.state.to_path_buf(),
                            ))
                            .into());
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
            graph: &graph,
            checksums: Mutex::new(checksums),
            detected: Mutex::new(Vec::new()),
        };

        let result = {
            // Nodes that must get visited during the traversal.
            let must_visit: IndexSet<_> = queue.iter().cloned().collect();

            // Create the subgraph from the queued nodes.
            let subgraph = Subgraph::new(
                &graph,
                graph.dfs(queue.into_iter()),
                graph.edges(),
            );

            // Build the subgraph.
            subgraph.traverse(
                |tid, index, node, events| {
                    build_node(&context, tid, index, node, events)
                },
                &must_visit,
                self.threads,
                false,
                self.event_sender.clone(),
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
            checksums,
            detected,
            ..
        } = context;
        let mut checksums = checksums.into_inner().unwrap();
        let detected = detected.into_inner().unwrap();

        // Add the detected inputs/outputs to the build graph. We must not
        // modify the build order when adding new edges to the graph. That is,
        // we can only add edges to *root* nodes. If we attempt to do otherwise,
        // then the build state shouldn't be committed.
        sync_detected(
            &mut graph,
            detected,
            &mut checksums,
            self.root,
            self.threads,
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
        .write_to_path(self.state)?;

        result.map_err(ErrorKind::TaskErrors)?;

        Ok(())
    }
}

fn build_node(
    context: &BuildContext<'_>,
    tid: usize,
    index: NodeIndex,
    node: &Node,
    events: &EventSender,
) -> Result<bool, Error> {
    match node {
        Node::Resource(r) => build_resource(context, tid, index, r, events),
        Node::Task(t) => build_task(context, tid, index, t, events),
    }
}

fn build_resource(
    context: &BuildContext<'_>,
    tid: usize,
    index: NodeIndex,
    node: &res::Any,
    events: &EventSender,
) -> Result<bool, Error> {
    let state = match node.state(context.root) {
        Ok(state) => state,
        Err(err) => {
            events.checksum_error(tid, node.clone(), &err);
            return Err(err);
        }
    };

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

/// Checks that the detected inputs or outputs are valid and won't change the
/// build order if added.
fn check_detected(
    graph: &BuildGraph,
    index: NodeIndex,
    detected: Detected,
    log: &mut dyn io::Write,
) -> Result<Detected, Error> {
    // It's only valid to add an implicit edge to a resource if:
    //  1. the resource does not exist,
    //  2. it's a root node, or
    //  3. an explicit edge from it already exists.
    let mut invalid_edges = Vec::new();

    for input in &detected.inputs {
        let node = Node::Resource(input.clone());
        if let Some(input) = graph.node_to_index(&node) {
            if !graph.contains_edge_by_index(input, index)
                && !graph.is_root_node(input)
            {
                invalid_edges.push((input, index));

                writeln!(
                    log,
                    "Error: '{}' must be added as an explicit dependency.",
                    node.as_res()
                )?;
            }
        }
    }

    if invalid_edges.is_empty() {
        Ok(detected)
    } else {
        Err(InvalidEdges(invalid_edges).into())
    }
}

fn build_task(
    context: &BuildContext<'_>,
    tid: usize,
    index: NodeIndex,
    node: &task::List,
    events: &EventSender,
) -> Result<bool, Error> {
    for task in node.iter() {
        let mut task_events = events.begin_task(tid, task.clone());

        if context.dryrun {
            let result: Result<_, &'static str> = Ok(Detected::new());
            task_events.finish(&result);
        } else {
            let result = task.execute(context.root, &mut task_events).and_then(
                |detected| {
                    // Check for detected edges that would change the build
                    // order. It's better to fail an individual task than the
                    // entire build in this case.
                    check_detected(
                        context.graph,
                        index,
                        detected,
                        &mut task_events,
                    )
                },
            );

            task_events.finish(&result);

            // Accumulate the detected inputs/outputs such that we can add them
            // to the implicit resources to the graph later. (We cannot modify
            // the build graph while traversing it.)
            context.detected.lock().unwrap().push((index, result?));
        }
    }

    Ok(true)
}
