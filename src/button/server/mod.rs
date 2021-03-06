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
mod client;
mod codec;
mod daemon;
mod error;
mod protocol;
mod service;
mod shutdown;
mod transport;

pub use client::Client;
pub use daemon::{connect_or_spawn, daemonize, run, run_daemon, try_connect};
pub use error::Error;
pub use protocol::{Request, Response};
pub use transport::Message;

use std::env;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use bincode;
use futures::{sync::mpsc, Future, Stream};
use log;
use serde::Serialize;
use tokio::net::TcpListener;

use service::ButtonService;
use shutdown::{Shutdown, ShutdownCause};

#[cfg(unix)]
fn notify_server_startup<T>(path: &Path, message: &T) -> Result<(), io::Error>
where
    T: Serialize,
{
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(path)?;

    bincode::serialized_size(message)
        .and_then(|size| bincode::serialize_into(&mut stream, &size))
        .and_then(|_| bincode::serialize_into(&mut stream, message))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

#[cfg(windows)]
fn notify_server_startup<T>(path: &Path, message: &T) -> Result<(), io::Error>
where
    T: Serialize,
{
    use std::fs::OpenOptions;

    let mut stream = OpenOptions::new().write(true).read(true).open(path)?;

    bincode::serialized_size(message)
        .and_then(|size| bincode::serialize_into(&mut stream, &size))
        .and_then(|_| bincode::serialize_into(&mut stream, message))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

pub struct Server {
    socket: TcpListener,
}

impl Server {
    pub fn new(port: u16) -> Result<Self, io::Error> {
        let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port);
        let result = Self::bind(&addr);

        // The client can set this environment variable to a Unix Domain Socket
        // path (on Unix) or to a named pipe (on Windows). We notify the client
        // that the server has been started by writing a message back to the
        // client. Then, the client can start making requests as soon as the
        // server is ready instead of retrying the connection on a loop.
        if let Some(path) = env::var_os("BUTTON_STARTUP_NOTIFY") {
            let message = match &result {
                Ok(server) => Ok(server.port()),
                Err(err) => Err(err.to_string()),
            };

            notify_server_startup(path.as_ref(), &message)?;
        }

        result
    }

    fn bind(addr: &SocketAddr) -> Result<Self, io::Error> {
        Ok(Server {
            socket: TcpListener::bind(addr)?,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }

    pub fn port(&self) -> u16 {
        self.addr().port()
    }

    pub fn run(self, idle: Duration) {
        let (tx, rx) = mpsc::channel(0);

        let service = ButtonService::new(tx);

        let timeout = Shutdown::new(idle, rx).map(|message| match message {
            ShutdownCause::Idle(duration) => {
                log::info!(
                    "Shutting down due to being idle for {:#?}.",
                    duration
                );
            }
            ShutdownCause::ShutdownRequested => {
                log::info!("Shutdown requested. Bye bye!");
            }
        });

        let server = self
            .socket
            .incoming()
            .map_err(|e| {
                log::error!("failed to accept socket; error = {:?}", e)
            })
            .for_each(move |socket| {
                let task = service.clone().bind(socket).map_err(|_| ());

                tokio::spawn(task)
            });

        let task = server.select2(timeout).map(|_| ()).map_err(|_| ());

        tokio::run(task);
    }
}
