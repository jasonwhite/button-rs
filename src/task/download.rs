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

use std::fmt;
use std::io;
use std::time;
use std::path::PathBuf;

use node::{Error, Task};

use retry;

/// A task to download a URL. This would normally be a task with no input
/// resources.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Download {
    /// Process and arguments to spawn.
    url: String,

    /// Expected SHA256 hash.
    sha256: String,

    /// Output path.
    path: PathBuf,

    /// How much time to give it to download. If `None`, there is no time limit.
    timeout: Option<time::Duration>,

    /// Retry settings.
    #[serde(default)]
    retry: retry::Retry,
}

impl Download {
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn new(url: String, sha256: String, path: PathBuf) -> Download {
        Download {
            url: url,
            sha256: sha256,
            path: path,
            timeout: None,
            retry: retry::Retry::new(),
        }
    }

    fn run_impl(&self, log: &mut io::Write) -> Result<(), Error> {
        writeln!(log, "Downloading `{:?}` to {:?}", self.url, self.path)?;

        // TODO: Download it to the given path.
        // TODO: Verify its checksum. Fail if it is wrong and delete the bad
        // file.
        // TODO: Implement timeouts and retries.
        Ok(())
    }
}

impl fmt::Display for Download {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl fmt::Debug for Download {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.url)
    }
}

impl Task for Download {
    fn run(&self, log: &mut io::Write) -> Result<(), Error> {
        self.retry.call(|| self.run_impl(log), retry::dummy_progress)
    }
}
