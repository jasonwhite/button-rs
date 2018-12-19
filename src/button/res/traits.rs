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
use std::hash::Hash;
use std::path::Path;

use crate::error::Error;

use serde::Serialize;

use crate::util;

pub type Checksum = util::Sha256;

/// The state associated with a resource. This is stored in the build state and
/// used to determine if a resource has changed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResourceState {
    /// The resource does not exist.
    Missing,

    /// The resource exists and we have the checksum of its contents.
    Checksum(Checksum),
}

impl fmt::Display for ResourceState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceState::Missing => write!(f, "missing"),
            ResourceState::Checksum(c) => write!(f, "{}", c),
        }
    }
}

/// A resource is an abstract representation of some unit of system state. A
/// resource can be a file, directory, environment variable. The main things we
/// are interested in doing with a resource is:
///
///  1. Getting its state so that we can determine if it has changed.
///  2. Deleting it when it is no longer needed.
///
/// A resource is merely an *identifier*. It should not store any state about
/// the actual thing it is referencing. The only state that can be stored with a
/// resource is `ResourceState`.
pub trait Resource:
    Serialize + Ord + PartialOrd + Eq + PartialEq + Hash + fmt::Display
{
    /// Gets the state of the resource. This is used to determine if it has
    /// changed.
    fn state(&self, root: &Path) -> Result<ResourceState, Error>;

    /// Deletes the resource. Care should be taken by the caller to not delete
    /// *input* resources. That is, resources that the build system did not
    /// produce. Deleting output resources is perfectly fine. There shall be no
    /// error if the resource already does not exist. Output resources should be
    /// deleting in reverse topological order such that files can get deleted
    /// before directories.
    fn delete(&self, root: &Path) -> Result<(), Error>;
}
