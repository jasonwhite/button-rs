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

//! In order to track changes to the build graph and nodes, we need to persist
//! the graph to disk. Each invocation of the build must read this in and do a
//! comparison to figure out what has changed since the last run.
//!
//! The persistent storage needs to store the following types of data:
//!
//!  1. **Nodes**
//!
//!     This is a generic data type and can be subdivided into two main types:
//!
//!      (a) Resources
//!      (b) Tasks
//!
//!     It should be possible to generically serialize these into a single SQL
//!     table. We are only interested in storing the node *identifiers* here.
//!     State associated with resources is stored in a separate table for
//!     reasons described later.
//!
//!     It may be better to split the resources and tasks into separate tables
//!     to further ensure that the graph remains bipartite. Note that, if this
//!     is done, the `edges` table and `pending` table must also be split up.
//!
//!  2. **Edges**
//!
//!     The edges between nodes must also be stored. This table is pretty
//!     trivial. We only need to store a pair of node IDs and some data
//!     associated with the edge (e.g., if it is an implicit or explicit edge).
//!
//!  3. **Resource State**
//!
//!     The resource state can be "Unknown", "Missing", or a 32-byte array. It
//!     is up to the implementation of the resource how the bytes in the array
//!     are interpreted. Most of the time, this will probably be a SHA256 hash.
//!     However, the 32-byte array can be packed however the implementation
//!     wishes. For example, tracking changes by timestamp may just choose to
//!     pack the buffer with a 64-bit timestamp, 64-bit length, and fill the
//!     rest with zeroes. In such a case, computing the checksum would be
//!     computationally wasteful.
//!
//!     Thus, this table is very simple: a node ID and a 32-byte array.
//!
//!     The first row in this table is special. It is the state of the build
//!     description. This is to avoid loading the build description if it hasn't
//!     changed.
//!
//!  4. **Pending Nodes**
//!
//!     This table shall contain a list of nodes that must be visited upon the
//!     next graph traversal. To understand why this is needed, consider the
//!     following scenario:
//!
//!      1. An initial build is started.
//!      2. The build fails.
//!      3. We fix the problem.
//!      4. We run the build again.
//!
//!     Now, we expect that only the failed tasks will get executed, not the
//!     entire build. Between steps (1) and (2), a number of important things
//!     must happen:
//!
//!      1. The stored build state is loaded from the database and into a graph
//!         data structure.
//!      2. The graph constructed from the list of rules is merged into the
//!         graph from (1). We do this by figuring out the "diff" between the
//!         two graphs (i.e., added/removed nodes and edges). Any new nodes must
//!         be visited in the build graph traversal.
//!      3. The updated build state is committed to disk.
//!
//!     After this point, the build can fail or get `kill -9`'d and things
//!     should be fine. However, the build system must not forget which nodes
//!     need to be visited in the traversal.


/// The storage medium for the build system that persists between builds,
/// implemented as a SQLite database. Also known as the build state.
///
/// The main operations we are interested in with this database are:
///
///  1. Loading or creating a database if it doesn't exist.
///  2. Adding nodes and edges
///  3. Creating a graph from the nodes and edges.
///  4. Getting the state associated with a node.
///  5. Getting the list of pending nodes.
///
struct Database {
    // TODO:
}
