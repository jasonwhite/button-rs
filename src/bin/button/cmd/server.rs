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
use std::time::Duration;

use button::{self, Error};
use humantime;
use log;
use pretty_env_logger;
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
}

impl Server {
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        let level = if let Some(level) = self.log_level {
            level
        } else {
            match env::var("BUTTON_LOG_LEVEL") {
                Ok(var) => var.parse::<log::LevelFilter>()?,
                Err(_) => log::LevelFilter::Info,
            }
        };

        // Initialize the logger.
        let mut builder = pretty_env_logger::formatted_timed_builder();
        builder.filter_module("button", level);
        builder.init();

        let server = button::server::Server::new(self.port)?;

        // Default of one hour.
        let idle = self.idle.unwrap_or(Duration::from_secs(60 * 60));

        log::info!(
            "Listening on {}. Will shutdown if idle for {}.",
            server.local_addr().unwrap(),
            humantime::format_duration(idle)
        );

        server.run(idle);

        Ok(())
    }
}
