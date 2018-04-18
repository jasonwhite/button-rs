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
use std::fs;
use std::io;
use std::path::PathBuf;

use super::traits::{Error, Task};

use res;
use retry;

/// A task to create a directory.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct MakeDir {
    /// Path to the directory to create.
    path: PathBuf,

    /// Retry settings.
    #[serde(default)]
    retry: retry::Retry,
}

impl MakeDir {
    pub fn new(path: PathBuf) -> MakeDir {
        MakeDir { path: path,
                  retry: retry::Retry::default(), }
    }

    fn execute_impl(&self, _log: &mut io::Write) -> Result<(), Error> {
        fs::create_dir_all(&self.path)
    }
}

impl fmt::Display for MakeDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mkdir {:?}", self.path)
    }
}

impl fmt::Debug for MakeDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for MakeDir {
    fn execute(&self, log: &mut io::Write) -> Result<(), Error> {
        self.retry.call(|| self.execute_impl(log), retry::progress_dummy)
    }

    fn known_outputs(&self, set: &mut res::Set) {
        set.insert(res::Dir::new(self.path.clone()).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use res;

    #[test]
    fn test_known_outputs() {
        let task = MakeDir::new(PathBuf::from("foobar"));
        let mut set = res::Set::new();
        task.known_outputs(&mut set);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&res::Dir::new("foobar".into()).into()));
    }
}
