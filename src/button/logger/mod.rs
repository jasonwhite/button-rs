// Copyright (c) 2018 Jason White
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
use std::fs;
use std::path::Path;

mod binary;
mod console;
pub mod events;
mod traits;

pub use self::binary::{Binary, BinaryTask, EventStream};
pub use self::console::{Console, ConsoleTask};
pub use self::traits::{EventLogger, LogResult, TaskLogger};

// TODO: Add additional loggers:
//
//  - web
//
//    A logger that sends all of the events to a web page for display. This
//    could be useful for seeing the output for long running tasks as they
//    occur. It could also display a Gantt chart of the build tasks.

/// A logger for writing to a file.
///
/// A stream of events (with timestamps) are written to the file. These can then
/// be read back later to "replay" the original build log through a text logger.
pub type BinaryFile = Binary<::std::io::BufWriter<::std::fs::File>>;

/// Creates a binary file logger from a path.
pub fn binary_file<P>(path: P) -> Result<BinaryFile, io::Error>
where
    P: AsRef<Path>
{
    let f = fs::File::create(path.as_ref())?;
    Ok(BinaryFile::from_writer(io::BufWriter::new(f)))
}

/// Types of loggers. Useful for static dispatch of multiple loggers.
pub enum Logger {
    Console(Console),
    BinaryFile(BinaryFile),
}
