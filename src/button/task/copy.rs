// Copyright (c) 2017 Jason White
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
use std::fmt;
use std::path::PathBuf;

use node::{Error, Task};

use retry;

/// A task to create a directory.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Copy {
    /// Path to copy from.
    from: PathBuf,

    /// Path to copy to.
    to: PathBuf,

    /// Retry settings.
    #[serde(default)]
    retry: retry::Retry,
}

impl Copy {
    fn execute_impl(&self, _log: &mut io::Write) -> Result<(), Error> {
        fs::copy(&self.from, &self.to)?;
        Ok(())
    }
}

impl fmt::Display for Copy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "copy {:?} -> {:?}", self.from, self.to)
    }
}

impl fmt::Debug for Copy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for Copy {
    fn execute(&self, log: &mut io::Write) -> Result<(), Error> {
        self.retry
            .call(|| self.execute_impl(log), retry::progress_dummy)
    }
}

