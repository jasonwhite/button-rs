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

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

use super::dir::Dir;
use super::file::File;
use super::traits::{Resource, ResourceState};

/// A set of resources.
pub type Set = BTreeSet<Any>;

/// Complete list of resource types. This list is used for deserialization
/// purposes.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum Any {
    File(File),
    Dir(Dir),
}

impl fmt::Display for Any {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Any::File(ref x) => x.fmt(f),
            Any::Dir(ref x) => x.fmt(f),
        }
    }
}

impl fmt::Debug for Any {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Any::File(ref x) => x.fmt(f),
            Any::Dir(ref x) => x.fmt(f),
        }
    }
}

impl<P: AsRef<Path>> From<P> for Any {
    fn from(path: P) -> Self {
        Any::File(File::new(path))
    }
}

impl From<File> for Any {
    fn from(res: File) -> Self {
        Any::File(res)
    }
}

impl From<Dir> for Any {
    fn from(res: Dir) -> Self {
        Any::Dir(res)
    }
}

impl Resource for Any {
    fn state(&self, root: &Path) -> Result<ResourceState, Error> {
        match self {
            Any::File(ref x) => x.state(root),
            Any::Dir(ref x) => x.state(root),
        }
    }

    fn delete(&self, root: &Path) -> Result<(), Error> {
        match self {
            Any::File(ref x) => x.delete(root),
            Any::Dir(ref x) => x.delete(root),
        }
    }
}
