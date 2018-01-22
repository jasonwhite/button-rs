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

use super::filepath::FilePath;
use super::traits::{Error, Resource, ResourceState};

use std::fmt;

/// Complete list of resource types. This list is used for deserialization
/// purposes.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum Any {
    FilePath(FilePath),
}

impl fmt::Display for Any {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Any::FilePath(ref x) => x.fmt(f),
        }
    }
}

impl fmt::Debug for Any {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Any::FilePath(ref x) => x.fmt(f),
        }
    }
}

impl From<FilePath> for Any {
    fn from(res: FilePath) -> Self {
        Any::FilePath(res)
    }
}

impl Resource for Any {
    fn state(&self) -> Result<ResourceState, Error> {
        match self {
            &Any::FilePath(ref x) => x.state(),
        }
    }

    fn delete(&self) -> Result<(), Error> {
        match self {
            &Any::FilePath(ref x) => x.delete(),
        }
    }
}
