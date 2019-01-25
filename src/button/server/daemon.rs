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
use std::fs;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;

use bincode;

use super::client::Client;
use super::error::Error;

// We need to be able to do the following things with the daemon process:
//  1. Check if it exists and connect to it.
//  2. Create it if it doesn't exist and connect to it.
//
// To do this, we just need a starting directory. The daemon shall run with its
// working directory at the root of the project (e.g., the directory which
// contains `button.json`).
//
//  - Read `{root}/.button/daemon` to check if the daemon is running. This file
//    will contain the PID and port that the daemon is running on.
//  - We can use the PID to kill the daemon if necessary.
//  - When starting up, the daemon will create `{root}/.button/daemon.lock` in
//    order to avoid race conditions with another instance of the daemon
//    starting. The lock file shall be renamed to `{root}/.button/daemon` when
//    initialization is complete and the server is ready to connect to.
//
// Creating the daemon:
//  1. After checking that another daemon is not running for the given root
//     directory, we create the daemon in a couple of different ways depending
//     on the platform.
//  2. On Unix platforms:
//     1. We create a socket for the daemon to notify when it has finished
//        starting up.
//     2. We `fork()`.
//
// Important files:
//  - .button/pid     File with PID (locked when the daemon process is running).
//  - .button/port    File with port number.
//  - .button/stderr  Redirected stderr.
//  - .button/stdout  Redirected stdout.

/// Connects to the daemon or spawns if it isn't running and then connects to
/// it.
///
/// If the daemon already exists, returns the port for the existing daemon.
pub fn connect_or_spawn(root: &Path) -> Result<Client, Error> {
    // Try connecting to the daemon.
    if let Ok(f) = fs::File::open(root.join(".button/port")) {
        let port = bincode::deserialize_from(f)?;
        let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port);

        if let Ok(client) = Client::connect(&addr) {
            return Ok(client);
        }
    }

    // Couldn't connect. Spawn the daemon!
    //
    // TODO: On Unix, create a temporary domain socket.
    // TODO: On Windows, create a named pipe.
    //
    // At the same time we're waiting for the above (with a timeout), spawn the
    // daemon process.  The daemon should automatically send a message over the
    // socket/pipe telling us the port to connect to.
    unimplemented!()
}

/// "Daemonizes" the process.
///
/// This does a couple of important things:
///  - Detaches the process from its parent process.
///  - Redirects stdout/stderr to a file such that logs can be viewed later.
///
/// This should only be called once we're ready to turn the current process into
/// a daemon.
#[cfg(unix)]
pub fn daemonize(root: &Path) -> Result<(), io::Error> {
    use daemonize::Daemonize;

    Daemonize::new()
        .pid_file(root.join(".button/pid"))
        .working_directory(root)
        .stdout(fs::File::create(".button/stdout")?)
        .stderr(fs::File::create(".button/stderr")?)
        .start()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // At this point, the process has forked 2 times and is now running as
    // a daemon. After this, the server can be started and start listening for
    // incoming connections.

    Ok(())
}

#[cfg(windows)]
pub fn daemonize(_root: &Path) -> Result<(), io::Error> {
    // Nothing to do on Windows. When the daemon process is spawned on Windows,
    // it is already detached.
    Ok(())
}
