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
use std::ffi::OsStr;
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;
use std::io;

use sha2::{Sha256, Digest};

use super::traits::{Resource, ResourceState, Error};


/// A directory resource. We don't care about the contents of this resource.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Dir {
    path: PathBuf,
}

impl Dir {
    pub fn new(path: PathBuf) -> Dir {
        Dir { path: path }
    }
}

impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for Dir {
    fn from(s: &'a T) -> Dir {
        Dir { path: PathBuf::from(s.as_ref()) }
    }
}

impl FromStr for Dir {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Dir { path: PathBuf::from(s) })
    }
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl Resource for Dir {
    fn state(&self) -> Result<ResourceState, Error> {
        match self.path.metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    // Use an empty hash to indicate existence.
                    Ok(ResourceState::Checksum(Sha256::default().result()))
                } else {
                    Err(io::Error::new(io::ErrorKind::Other, "Not a directory"))
                }
            }
            Err(err) => {
                match err.kind() {
                    io::ErrorKind::NotFound => Ok(ResourceState::Missing),
                    _ => Err(err),
                }
            }
        }
    }

    /// Deletes the directory if it is empty. Resources are deleted in reverse
    /// topological order. Thus, if all output resource are accounted for,
    /// directory deletion will always succeed.
    fn delete(&self) -> Result<(), Error> {
        match fs::remove_dir(&self.path) {
            Ok(()) => Ok(()),
            Err(err) => {
                match err.kind() {
                    // Don't care if it doesn't exist.
                    io::ErrorKind::NotFound => Ok(()),
                    _ => Err(err),
                }
            }
        }
    }
}