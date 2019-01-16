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

use serde::{Deserialize, Serialize};

use crate::detect::Detected;
use crate::error::Error;
use crate::res;
use crate::util::{progress_dummy, Retry};

use super::traits::Task;

/// A task to create a directory.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
pub struct Copy {
    /// Path to copy from.
    from: PathBuf,

    /// Path to copy to.
    to: PathBuf,

    /// Retry settings.
    retry: Option<Retry>,
}

impl Copy {
    fn execute_impl(
        &self,
        root: &Path,
        _log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        fs::copy(&root.join(&self.from), &root.join(&self.to))?;
        Ok(Detected::new())
    }
}

impl fmt::Display for Copy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "copy {:?} -> {:?}", self.from, self.to)
    }
}

impl fmt::Debug for Copy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Task for Copy {
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
        // Depend on the output directory existing. This ensures that any
        // necessary directory creation happens first.
        if let Some(parent) = self.to.parent() {
            if parent != Path::new("") && parent != Path::new(".") {
                set.insert(res::Dir::new(parent).into());
            }
        }

        set.insert(self.from.clone().into());
    }

    fn known_outputs(&self, set: &mut res::Set) {
        set.insert(self.to.clone().into());
    }
}
