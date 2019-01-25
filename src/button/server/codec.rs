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

use bytes::BytesMut;
use tokio::codec::{length_delimited::LengthDelimitedCodec, Decoder, Encoder};

use super::error::Error;

/// Shim for getting the right error type that we need.
#[derive(Debug)]
pub struct Codec(LengthDelimitedCodec);

impl Codec {
    pub fn new() -> Codec {
        Codec(LengthDelimitedCodec::new())
    }
}

impl Decoder for Codec {
    type Item = <LengthDelimitedCodec as Decoder>::Item;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.0.decode(src)?)
    }
}

impl Encoder for Codec {
    type Item = <LengthDelimitedCodec as Encoder>::Item;
    type Error = Error;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        self.0.encode(item, dst)?;
        Ok(())
    }
}
