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

use tokio::{
    codec::Framed,
    io::{AsyncRead, AsyncWrite},
    prelude::*,
};
use tokio_serde_bincode::{ReadBincode, WriteBincode};
use tower_service::Service;

use crate::codec::Codec;
use crate::error::Error;
use crate::protocol::{Request, Response};

use futures::{sync::mpsc::Receiver, try_ready, Poll, Sink, StartSend, Stream};

/// Reimplementation of `futures::future::Either` to get a `Stream`
/// implementation.
enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B> Future for Either<A, B>
where
    A: Future,
    B: Future<Item = A::Item, Error = A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<A::Item, A::Error> {
        match *self {
            Either::A(ref mut a) => a.poll(),
            Either::B(ref mut b) => b.poll(),
        }
    }
}

impl<A, B> Stream for Either<A, B>
where
    A: Stream,
    B: Stream<Item = A::Item, Error = A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<Option<A::Item>, A::Error> {
        match *self {
            Either::A(ref mut a) => a.poll(),
            Either::B(ref mut b) => b.poll(),
        }
    }
}

enum Frame<R, B> {
    /// This frame is part of a body. If `None`, then there are no more body
    /// frames.
    Body(Option<B>),

    /// This frame contains a message. If the second parameter is `true`, then
    /// one or more body frames follow.
    Message(R, bool),
}

enum Message<R, B> {
    /// A response frame followed by one or more body frames.
    WithBody(R, B),

    /// Just a response frame, no body.
    WithoutBody(R),
}

impl<R, B> Message<R, B> {
    pub fn into_request(self) -> R {
        match self {
            Message::WithBody(r, _) => r,
            Message::WithoutBody(r) => r,
        }
    }
}

struct Transport<I>
where
    I: AsyncRead + AsyncWrite,
{
    inner: WriteBincode<ReadBincode<Framed<I, Codec>, Request>, Response>,
}

impl<I> Transport<I>
where
    I: AsyncRead + AsyncWrite,
{
    pub fn new(io: I) -> Self {
        Transport {
            inner: WriteBincode::new(ReadBincode::new(Framed::new(
                io,
                Codec::new(),
            ))),
        }
    }
}

impl<I> Stream for Transport<I>
where
    I: AsyncRead + AsyncWrite,
{
    type Item = Message<Request, Receiver<()>>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Transform the request into a Message without a body. Only responses
        // can have a body.
        let msg = try_ready!(self.inner.poll());
        Ok(Async::Ready(msg.map(|m| Message::WithoutBody(m))))
    }
}

impl<I> Sink for Transport<I>
where
    I: AsyncRead + AsyncWrite,
{
    type SinkItem = Frame<Response, Response>;
    type SinkError = Error;

    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> StartSend<Self::SinkItem, Self::SinkError> {
        match item {
            Frame::Message(message, body) => {
                match self.inner.start_send(message)? {
                    AsyncSink::Ready => Ok(AsyncSink::Ready),
                    AsyncSink::NotReady(message) => {
                        Ok(AsyncSink::NotReady(Frame::Message(message, body)))
                    }
                }
            }
            Frame::Body(Some(chunk)) => match self.inner.start_send(chunk)? {
                AsyncSink::Ready => Ok(AsyncSink::Ready),
                AsyncSink::NotReady(chunk) => {
                    Ok(AsyncSink::NotReady(Frame::Body(Some(chunk))))
                }
            },
            Frame::Body(None) => Ok(AsyncSink::Ready),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.close()
    }
}

#[derive(Clone)]
pub struct ButtonService;

type ServerRequest = Message<Request, Receiver<()>>;
type ServerResponse = Message<Response, Receiver<Response>>;

impl Service<ServerRequest> for ButtonService {
    type Response = ServerResponse;
    type Error = Error;
    type Future =
        Box<Future<Item = Self::Response, Error = Self::Error> + Send>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: ServerRequest) -> Self::Future {
        // TODO: Put all server logic here.
        println!("Got request: {:?}", req.into_request());
        Box::new(future::ok(Message::WithoutBody(Response)))
    }
}

impl ButtonService {
    pub fn new() -> Self {
        ButtonService
    }

    pub fn bind<T>(mut self, io: T) -> impl Future<Item = (), Error = Error>
    where
        T: AsyncRead + AsyncWrite,
    {
        let (sink, stream) = Transport::new(io).split();

        let sink = sink.sink_from_err::<Error>();

        stream
            .from_err::<Error>()
            .and_then(move |input| self.call(input))
            .and_then(|message| {
                let f = match message {
                    Message::WithoutBody(message) => Either::A(stream::once(
                        Ok(Frame::Message(message, false)),
                    )),
                    Message::WithBody(message, body) => Either::B(
                        stream::once(Ok(Frame::Message(message, true)))
                            .chain(body.map(|chunk| Frame::Body(Some(chunk))))
                            .chain(stream::once(Ok(Frame::Body(None)))),
                    ),
                };
                Ok(f.from_err::<Error>())
            })
            .flatten()
            .forward(sink)
            .map(|_| ())
    }
}
