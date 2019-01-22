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

/// A request for the server.
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    /// Requests a build.
    Build,

    /// Requests a clean.
    Clean,

    /// Requests the server to shut down.
    Shutdown,

    /// Requests a build to happen automatically whenever a file is changed.
    Watch,
}

/// The response sent back to the client afer a request is made.
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Success,
    Shutdown,
}

/// An item in the response body. Zero or more items may follow a response.
#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseItem {
    /// A build event. Sent by the server when a build event occurs.
    BuildEvent(usize),
}
