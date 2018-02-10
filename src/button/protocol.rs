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

//! The client/server request and response protocol. This is a streaming
//! protocol.


/// A client request.
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    /// Requests a new build. The client should then listen for a series of
    /// responses from the server about the progress of the build.
    Build(Build),

    /// Requests the build graph.
    Graph,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Build {
    // TODO: Send the build graph to the server.
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BuildEvent {
    Started,

    /// The build has finished.
    ///
    /// TODO: Include:
    ///  * Duration of the build.
    ///  *
    Finished,
}

/// A server response.
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    /// Response for `Request::Build`.
    ///
    /// Indicates that a build has been started. Following this, the client
    /// should then listen for `BuildEvent` responses.
    Building,

    /// Response for `Request::Graph`.
    Graph,
}
