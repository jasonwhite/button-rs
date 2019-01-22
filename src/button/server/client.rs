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
use std::net::SocketAddr;

use futures::{Future, Sink, Stream};
use tokio::net::TcpStream;

use super::error::Error;
use super::protocol::{Request, Response, ResponseItem};
use super::transport::{Frame, Message, Transport};

/// A connection to the server.
pub struct Client {
    stream: Transport<TcpStream, Frame<Response, ResponseItem>, Request>,
}

impl Client {
    pub fn connect(
        addr: &SocketAddr,
    ) -> impl Future<Item = Self, Error = Error> {
        TcpStream::connect(addr)
            .from_err::<Error>()
            .map(|tcp| Client {
                stream: Transport::new(tcp),
            })
    }

    pub fn request(
        self,
        request: Request,
    ) -> impl Future<
        Item = Message<
            Response,
            impl Stream<Item = ResponseItem, Error = Error>,
        >,
        Error = Error,
    > {
        self.stream.send(request).and_then(move |stream| {
            stream
                .into_future()
                .map_err(|(e, _)| e)
                .map(|(item, stream)| match item.unwrap() {
                    Frame::Message(r, has_body) => {
                        if has_body {
                            let body_stream = stream
                                .take_while(|frame| match frame {
                                    Frame::Body(Some(_)) => Ok(true),
                                    _ => Ok(false),
                                })
                                .map(|frame| match frame {
                                    Frame::Body(Some(b)) => b,
                                    _ => unreachable!(),
                                });

                            Message::WithBody(r, body_stream)
                        } else {
                            Message::WithoutBody(r)
                        }
                    }
                    Frame::Body(_) => unreachable!(),
                })
        })
    }
}
