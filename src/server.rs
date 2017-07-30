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

use graph::{Node, BuildGraph};
use std::path::Path;

/// The state of the build. This is the primary data structure the server
/// operates on and it persists through individual builds.
///
/// The build system follows a client/server model.
///
/// The client's responsibilities include:
///
///  1. Starting up the server if it isn't running.
///  2. Telling the server to start a build. The server is given a path to the
///     rules JSON file or it uses the default path.
///  3. Waiting for the event stream from the server.
///  4. Forwarding signals such as SIGINT to the server to kill the build.
///
/// The server's responsibilities include:
///
///  1. Holding state about the build. This includes: the build graph, the state
///     of all resources (i.e., their checksums), and the queue of nodes that
///     need to be visited.
///  2. Doing all of the heavy lifting such as actually running the build, doing
///     cleans, etc.
///  3. Sending the clients an event stream. This event stream shall be a log of
///     every action that occurs in the build. If desired, this can be saved and
///     replayed later.
///
/// This client/server architecture has some major advantages:
///
///  1. Keeping the build graph in memory is very fast.
///  2. The server can watch for changes to files and add them as pending nodes
///     to be built next time.
///  3. The server can have multiple clients. For example, one client could be a
///     command line client and another could be a web server.
pub struct Server {
    /// Append-only list of nodes.
    nodes: Vec<Node>,

    /// Map of nodes to indices.
    node_indices: HashMap<Node, usize>,

    /// The nodes that have just been added to the build graph and need to be
    /// visited upon the next build. For example, if we add a new task to the
    /// build graph, it should get executed no matter where it is in the build
    /// graph.
    pending: Vec<usize>,

    /// The build graph, including all implicit edges.
    graph: BuildGraph,

    // TODO: Keep the current state of all resources. This shall simply be a
    // hash map of `Resource` to `ResourceState`.
}

impl Server {
    pub fn new() -> Server {
        Server {
            pending: Vec::new(),
            graph: BuildGraph::new(),
        }
    }

    /// Runs a build.
    pub fn build(&self,
                 rules: &Path,
                 threads: usize,
                 dryrun: bool)
                 -> Result<(), String> {
        Ok(())
    }
}
