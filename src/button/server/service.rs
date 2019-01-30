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

use futures::{sync::mpsc, Poll};
use log;
use tokio::{
    self,
    io::{AsyncRead, AsyncWrite},
    prelude::*,
};
use tower_service::Service;

use crate::rules::Rules;
use crate::util::Either;

use super::error::Error;
use super::protocol::{BodyItem, Request, Response};
use super::shutdown::ShutdownMessage;
use super::transport::{Frame, Message, Transport};

/// The service. This is instantiated for each connection to the server.
#[derive(Clone)]
pub struct ButtonService {
    /// Channel to reset the idle timer so the server doesn't shut down.
    shutdown: mpsc::Sender<ShutdownMessage>,
}

impl ButtonService {
    pub fn new(shutdown: mpsc::Sender<ShutdownMessage>) -> Self {
        ButtonService { shutdown }
    }

    pub fn bind<T>(mut self, io: T) -> impl Future<Item = (), Error = Error>
    where
        T: AsyncRead + AsyncWrite,
    {
        let (sink, source) = Transport::new(io).split();

        let mut shutdown = self.shutdown.clone();

        let sink = sink.sink_from_err::<Error>().with(
            move |item| -> Result<_, Error> {
                // Reset the idle timer to keep the server alive while sending
                // back a (potentially long) body.
                drop(shutdown.start_send(ShutdownMessage::ResetIdle));
                Ok(item)
            },
        );

        source
            .from_err::<Error>()
            .and_then(move |input| self.call(input))
            .map(|message| {
                match message {
                    Message::WithoutBody(message) => Either::A(stream::once(
                        Ok(Frame::Message(message, false)),
                    )),
                    Message::WithBody(message, body) => Either::B(
                        stream::once(Ok(Frame::Message(message, true)))
                            .chain(body.map(|chunk| Frame::Body(Some(chunk))))
                            .chain(stream::once(Ok(Frame::Body(None)))),
                    ),
                }
                .from_err::<Error>()
            })
            .flatten()
            .forward(sink)
            .map(|_| ())
    }

    /// Handles a 'build' request.
    fn build(&mut self) -> <Self as Service<Request>>::Future {
        let (tx, rx) = mpsc::channel(1);

        // Send some events.
        tokio::spawn(
            tx.send(BodyItem::BuildEvent(1))
                .and_then(|tx| tx.send(BodyItem::BuildEvent(2)))
                .map_err(|_| ())
                .map(|_| ()),
        );

        let message = Message::WithBody(Ok(()), rx);

        Box::new(future::ok(message))
    }

    /// Handles a 'clean' request.
    fn clean(&mut self) -> <Self as Service<Request>>::Future {
        Box::new(future::ok(Message::WithoutBody(Ok(()))))
    }

    /// Handles an 'update' request.
    fn update(&mut self, _rules: Rules) -> <Self as Service<Request>>::Future {
        Box::new(future::ok(Message::WithoutBody(Ok(()))))
    }

    /// Handles a 'watch' request.
    fn watch(&mut self) -> <Self as Service<Request>>::Future {
        Box::new(future::ok(Message::WithoutBody(Ok(()))))
    }

    /// Handles a 'shutdown' request.
    fn shutdown(&mut self) -> <Self as Service<Request>>::Future {
        Box::new(
            self.shutdown
                .clone()
                .send(ShutdownMessage::Shutdown)
                .from_err::<Error>()
                .map(|_| Message::WithoutBody(Ok(()))),
        )
    }
}

impl Service<Request> for ButtonService {
    type Response = Message<Response, mpsc::Receiver<BodyItem>>;
    type Error = Error;
    type Future =
        Box<Future<Item = Self::Response, Error = Self::Error> + Send>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        log::info!("Got request: {:?}", request);

        // We received a request. Keep the server alive. Note that we don't care
        // if `start_send` cannot complete the send. If the channel's buffer is
        // full due to outstanding requests, then sending another reset message
        // isn't necessary.
        drop(self.shutdown.start_send(ShutdownMessage::ResetIdle));

        match request {
            Request::Build => self.build(),
            Request::Clean => self.clean(),
            Request::Update(rules) => self.update(rules),
            Request::Watch => self.watch(),
            Request::Shutdown => self.shutdown(),
        }
    }
}
