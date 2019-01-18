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

//! This is the Button server process. It does several things:
//!  1. Watches for file changes to both input and output resources, hashes
//!     them, and holds a set of modified resources ready to go whenever a build
//!     is requested.
//!  2. Holds all of the build state. By holding all of the state in memory, it
//!     doesn't need to be re-read for every build. It should be written to disk
//!     asynchronously after each build to prevent data loss.
//!  3. Produces a stream of events for the client that requested a build. The
//!     client can use this event stream to display the build output.

mod codec;
mod error;
mod protocol;
mod service;

use std::{
    env, net::SocketAddr, num::NonZeroUsize, path::PathBuf, time::Duration,
};
use tokio::{self, net::TcpListener, prelude::*};

use serde::{Deserialize, Serialize};

use crate::service::ButtonService;

#[allow(dead_code)]
struct WatchRequest {
    /// Path to the rules. We will also watch for changes to this file and
    /// update the build graph as necessary.
    rules: PathBuf,

    /// The amount of time to wait before building. The timeout is reset every
    /// time a new change is seen. Useful when source code is changed by a tool
    /// (e.g., git checkout, code formatting).
    timeout: Duration,
}

#[allow(dead_code)]
struct BuildRequest {
    /// Path to the build rules.
    rules: PathBuf,

    /// The number of threads to use. Defaults to the number of logical cores.
    threads: Option<NonZeroUsize>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Request {
    /// Requests a build, using the path to the build rules.
    Build,

    /// Requests a build to happen automatically whenever a file is changed.
    Watch,
}

#[derive(Debug, Serialize, Deserialize)]
enum Response {}

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:0".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let socket = TcpListener::bind(&addr)?;

    if let Ok(addr) = socket.local_addr() {
        println!("Listening on: {}", addr);
    }

    let service = ButtonService::new();

    let done = socket
        .incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(move |socket| {
            let task = service.clone().bind(socket).map_err(|_| ());

            tokio::spawn(task)
        });

    tokio::run(done);
    Ok(())
}
