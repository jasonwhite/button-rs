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
use std::io::{self, Write as _};

use bincode;

use super::{Event, EventHandler, Timestamp};

/// Serializes events to a file. This is useful for being able to replay events
/// later through a different event handler.
pub struct Binary {
    writer: io::BufWriter<fs::File>,
}

impl Binary {
    pub fn new(file: fs::File) -> Self {
        Binary {
            writer: io::BufWriter::new(file),
        }
    }
}

impl EventHandler for Binary {
    type Error = bincode::Error;

    fn call(
        &mut self,
        timestamp: Timestamp,
        event: Event,
    ) -> Result<(), Self::Error> {
        bincode::serialize_into(&mut self.writer, &(timestamp, event))?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()?;
        Ok(())
    }
}
