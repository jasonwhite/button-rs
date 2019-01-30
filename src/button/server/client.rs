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
use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use futures::{Future, Sink, Stream};
use tokio::{net::TcpStream, runtime::current_thread::Runtime};

use super::error::Error;
use super::protocol::{BodyItem, Request, Response};
use super::transport::{Frame, Transport};

/// A connection to the server.
pub struct Client {
    rt: Runtime,
    stream: Transport<TcpStream, Frame<Response, BodyItem>, Request>,
}

impl Client {
    pub fn new(port: u16) -> Result<Self, Error> {
        let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port);
        Self::connect(&addr)
    }

    pub fn connect(addr: &SocketAddr) -> Result<Self, Error> {
        let mut rt = Runtime::new()?;

        let tcp = rt.block_on(TcpStream::connect(addr))?;

        Ok(Client {
            rt,
            stream: Transport::new(tcp),
        })
    }

    /// The client's address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.get_ref().local_addr()
    }

    /// The server's address.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.get_ref().peer_addr()
    }

    /// The port of the server we're connected to.
    pub fn port(&self) -> u16 {
        self.peer_addr().unwrap().port()
    }

    /// Makes a request and returns the response and a boolean indicating the
    /// presence of a body. If the response contains a body, the body *must* be
    /// fully read in order to make another request.
    pub fn request(
        &mut self,
        request: Request,
    ) -> Result<(Response, bool), Error> {
        // TODO: This is not truely async due to the complexity of reusing
        // connections. Such a scheme requires maintaining a connection pool.
        // When a response is fully read, the connection is put back into the
        // pool to be reused by a future request. This is how Hyper works.
        let stream = &mut self.stream;

        self.rt
            .block_on(stream.send(request).and_then(move |stream| {
                stream.into_future().map_err(|(e, _)| e).map(
                    |(item, _stream)| {
                        if let Frame::Message(r, has_body) = item.unwrap() {
                            (r, has_body)
                        } else {
                            unreachable!()
                        }
                    },
                )
            }))
    }

    /// Reads a single body item. Returns `None` when there are no more body
    /// items.
    pub fn read_body_item(&mut self) -> Result<Option<BodyItem>, Error> {
        let stream = &mut self.stream;

        self.rt
            .block_on(stream.into_future().map_err(|(e, _)| e).map(
                |(item, _stream)| {
                    if let Some(Frame::Body(b)) = item {
                        b
                    } else {
                        None
                    }
                },
            ))
    }
}
