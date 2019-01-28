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

use futures::{Poll, Sink, StartSend, Stream};
use serde::{Deserialize, Serialize};
use tokio::{
    codec::Framed,
    io::{AsyncRead, AsyncWrite},
};
use tokio_serde_bincode::{ReadBincode, WriteBincode};

use super::codec::Codec;
use super::error::Error;

#[derive(Serialize, Deserialize)]
pub enum Frame<R, B> {
    /// This frame is part of a body. If `None`, then there are no more body
    /// frames.
    Body(Option<B>),

    /// This frame contains a message. If the second parameter is `true`, then
    /// one or more body frames follow.
    Message(R, bool),
}

pub enum Message<R, B> {
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

/// Helper for sending/receiving arbitrary serializable/deserializable structs
/// using bincode.
///
/// A more complicated protocol can be built on top of this, such as one that
/// sends or receives a message with a body attached to it.
pub struct Transport<T, R, W>
where
    T: AsyncRead + AsyncWrite,
{
    inner: WriteBincode<ReadBincode<Framed<T, Codec>, R>, W>,
}

impl<T, R, W> Transport<T, R, W>
where
    T: AsyncRead + AsyncWrite,
    R: for<'de> Deserialize<'de>,
    W: Serialize,
{
    pub fn new(stream: T) -> Self {
        Transport {
            inner: WriteBincode::new(ReadBincode::new(Framed::new(
                stream,
                Codec::new(),
            ))),
        }
    }

    pub fn get_ref(&self) -> &T {
        self.inner.get_ref().get_ref().get_ref()
    }
}

impl<T, R, W> Stream for Transport<T, R, W>
where
    T: AsyncRead + AsyncWrite,
    R: for<'de> Deserialize<'de>,
{
    type Item = R;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.poll()
    }
}

impl<T, R, W> Sink for Transport<T, R, W>
where
    T: AsyncRead + AsyncWrite,
    W: Serialize,
{
    type SinkItem = W;
    type SinkError = Error;

    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.close()
    }
}
