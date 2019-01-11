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
use std::path::{Path, PathBuf};

use super::traits::Task;
use crate::detect::Detected;
use crate::error::Error;

use crate::res;
use crate::util::{progress_dummy, Retry};

/// A task to create a directory.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
pub struct MakeDir {
    /// Path to the directory to create.
    path: PathBuf,

    /// Retry settings.
    retry: Option<Retry>,
}

impl MakeDir {
    pub fn new(path: PathBuf) -> MakeDir {
        MakeDir { path, retry: None }
    }

    fn execute_impl(
        &self,
        root: &Path,
        _log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        // Only create the last directory, not the entire directory path. We
        // would not be able to properly clean up directories if we did the
        // equivalent of `mkdir -p`. Instead, the entire directory path chain
        // should be coded into the build graph. For example, to create the
        // directory path `obj/x64`, there should be two rules: one for creating
        // `obj` and one for creating `obj/x64`. We automatically add a
        // dependency on the parent path such that they get created in the
        // correct order.
        match fs::create_dir(&root.join(&self.path)) {
            Ok(()) => Ok(()),
            Err(err) => match err.kind() {
                // Don't care if it already exists.
                io::ErrorKind::AlreadyExists => Ok(()),
                _ => Err(err),
            },
        }?;

        Ok(Detected::new())
    }
}

impl fmt::Display for MakeDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mkdir {:?}", self.path)
    }
}

impl fmt::Debug for MakeDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for MakeDir {
    fn execute(
        &self,
        root: &Path,
        log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        if let Some(retry) = &self.retry {
            retry.call(|| self.execute_impl(root, log), progress_dummy)
        } else {
            self.execute_impl(root, log)
        }
    }

    fn known_inputs(&self, set: &mut res::Set) {
        // Add parent directory as an input. Adding this dependency also ensures
        // that we delete directories in the correct order relative to one
        // another.
        if let Some(parent) = self.path.parent() {
            if parent != Path::new("") && parent != Path::new(".") {
                set.insert(res::Dir::new(parent).into());
            }
        }
    }

    fn known_outputs(&self, set: &mut res::Set) {
        set.insert(res::Dir::new(self.path.clone()).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::res;

    #[test]
    fn known_inputs_parent() {
        let task = MakeDir::new(PathBuf::from("foo/bar"));
        let mut set = res::Set::new();
        task.known_inputs(&mut set);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&res::Dir::new("foo").into()));
    }

    #[test]
    fn known_inputs_no_parent() {
        let task = MakeDir::new(PathBuf::from("foo"));
        let mut set = res::Set::new();
        task.known_inputs(&mut set);
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn known_inputs_dot_parent() {
        let task = MakeDir::new(PathBuf::from("./foo"));
        let mut set = res::Set::new();
        task.known_inputs(&mut set);
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn known_outputs() {
        let task = MakeDir::new(PathBuf::from("foobar"));
        let mut set = res::Set::new();
        task.known_outputs(&mut set);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&res::Dir::new("foobar").into()));
    }
}
