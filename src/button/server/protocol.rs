// Copyright (c) 2019 Jason White
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
use serde::{Deserialize, Serialize};

use crate::rules::Rules;

/// A request for the server.
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    /// Requests a build.
    Build,

    /// Requests that all output resources get deleted.
    Clean,

    /// Updates the build graph with the on-disk rules.
    Update(Rules),

    /// Requests a build to happen automatically whenever a file is changed.
    Watch,

    /// Requests the server to shut down.
    Shutdown,
}

/// A generic response that is sent when an error occurs.
#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseError {
    Other(String),
}

/// The response sent back to the client afer a request is made. This is just an
/// alias for `Result` to make handling errors that come from the server easier.
pub type Response = Result<(), ResponseError>;

/// An item in the response body. Zero or more items may follow a response.
#[derive(Serialize, Deserialize, Debug)]
pub enum BodyItem {
    /// A build event. Sent by the server when a build event occurs.
    BuildEvent(usize),
}
