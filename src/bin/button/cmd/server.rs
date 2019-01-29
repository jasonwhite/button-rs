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

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use button::{self, Error, ErrorKind};
use humantime;
use log;
use structopt::StructOpt;

use crate::opts::GlobalOpts;

#[derive(StructOpt, Default, Debug)]
pub struct Server {
    /// Port to use. By default a random, unused port is used.
    #[structopt(long = "port", short = "p", default_value = "0")]
    port: u16,

    /// The amount of time to wait after the server has become idle before
    /// shutting down.
    #[structopt(
        long = "idle-timeout",
        parse(try_from_str = "humantime::parse_duration")
    )]
    idle: Option<Duration>,

    /// Logging level to use.
    #[structopt(long = "log-level")]
    log_level: Option<log::LevelFilter>,

    /// Make this a daemon process.
    #[structopt(long = "--daemon")]
    daemon: bool,

    /// Root directory for the daemon. Defaults to the current working
    /// directory.
    #[structopt(long = "root", parse(from_os_str))]
    root: Option<PathBuf>,

    /// Run the server in the foreground?
    #[structopt(long = "foreground", short = "f")]
    foreground: bool,
}

impl Server {
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        if self.daemon {
            button::server::run_daemon(self.port, self.idle, self.log_level)?;
        } else if self.foreground {
            // Can't run in the foreground if there is already a server running.
            if let Some(client) = button::server::try_connect(".")? {
                return Err(ErrorKind::Other(
                    format!(
                        "Server is already running on port {}",
                        client.port()
                    )
                    .into(),
                )
                .into());
            }

            if let Some(dir) = &self.root {
                env::set_current_dir(dir)?;
            }

            button::server::run(self.port, self.idle, self.log_level)?;
        } else {
            let root = self
                .root
                .as_ref()
                .map(PathBuf::as_path)
                .unwrap_or_else(|| Path::new("."));

            let client = button::server::connect_or_spawn(root, || {
                // Run `button server --daemon` in order to spawn the daemon.
                let mut command = Command::new(env::current_exe()?);
                command.arg("server");
                command.arg("--daemon");
                command.args(&["--port", &self.port.to_string()]);

                if let Some(log_level) = &self.log_level {
                    command.args(&["--log-level", &log_level.to_string()]);
                }

                if let Some(idle) = &self.idle {
                    command.args(&[
                        "--idle-timeout",
                        &humantime::format_duration(*idle).to_string(),
                    ]);
                }

                Ok(command)
            })?;

            println!("Server is running at {}.", client.peer_addr().unwrap());
        }

        Ok(())
    }
}
