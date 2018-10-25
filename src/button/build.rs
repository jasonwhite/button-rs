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

use build_graph::{BuildGraph, FromRules, Node};
use detect::Detected;
use logger::{EventLogger, TaskLogger};
use res::{self, Resource, ResourceState};
use rules::Rules;
use state::BuildState;
use task::{self, Task};

use graph::{Algo, Indexable, Neighbors, NodeIndex, Nodes, Subgraph};

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

/// For a list of nodes, delete them in reverse topological order.
fn delete_nodes<I, L>(
    root: &Path,
    state: &BuildState,
    nodes: I,
    threads: usize,
    logger: &L,
) -> Result<(), Error>
where
    I: Iterator<Item = NodeIndex>,
    L: EventLogger,
{
    let removed: HashSet<_> = nodes.collect();

    state
        .graph
        .traverse(
            |tid, index, node| {
                if let Node::Resource(r) = node {
                    // Only delete the resource if its in our set of removed
                    // resources and if the state has been computed. A computed
                    // state indicates that the build system "owns" the
                    // resource.
                    if removed.contains(&index)
                        && state.checksums.contains_key(&index)
                    {
                        logger.delete(tid, r)?;
                        r.delete(root)?;
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
                        if !dryrun
                            && !state.graph.is_root_node(index)
                            && state.checksums.contains_key(&index)
                        {
                            logger.delete(tid, r)?;
                            r.delete(self.root)?;
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
            graph,
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
                    let (old_state, removed) = state.update(graph);
                    if !removed.is_empty() && !dryrun {
                        // TODO: For a dryrun, print out the resources that
                        // would be deleted.
                        delete_nodes(
                            self.root,
                            &old_state,
                            removed.into_iter(),
                            threads,
                            logger,
                        )
                        .context("Failed deleting resources")?;
                    }

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

        for node in DirtyNodes::new(self.root, &graph, &checksums) {
            queue.push(node);
        }

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
            let subgraph = Subgraph::new(&graph, graph.dfs(queue.into_iter()));

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
                // Queue all failed nodes so that they get visited again next
                // time.
                errors.iter().map(|x| x.0).collect()
            } else {
                Vec::new()
            }
        };

        // TODO: Add the detected inputs/outputs to the build graph. We must not
        // modify the build order when adding new edges to the graph. That is,
        // we can only add edges to *root* nodes. If we attempt to do otherwise,
        // then the build state shouldn't be committed.

        // Serialize the state. This must be the last thing that we do. If
        // anything fails above (e.g., failing to delete a resource), the state
        // will remain untouched and the error should be reproducible. Note that
        // task failures should not prevent the state from being saved. Instead,
        // those are added to the queue to be executed again.
        BuildState {
            graph,
            queue,
            checksums: context.checksums.into_inner().unwrap(),
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
