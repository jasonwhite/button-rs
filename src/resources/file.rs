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

use serde::{de, Deserialize, Deserializer};

use resource::{Resource, Checksum, Error};

use std::path::PathBuf;

/// A file resource. This can actually be a file *or* directory.
#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct File {
    pub path: PathBuf,
}

impl File {
    #[allow(dead_code)]
    fn new(path: PathBuf) -> File {
        File { path: path }
    }
}

impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for File {
    fn from(s: &'a T) -> File {
        File { path: PathBuf::from(s.as_ref()) }
    }
}

impl FromStr for File {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(File {
            path: PathBuf::from(s)
        })
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

// Derserialize a `File` from a string.
impl<'de> Deserialize<'de> for File {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl Resource for File {
    /// If a file, the checksum is of the contents of the file. If a directory,
    /// the checksum is of the sorted list of directory entries. Thus, if a file
    /// is added or removed from a directory, the checksum changes.
    fn checksum(&self) -> Result<Checksum, Error> {
        unimplemented!()
    }

    /// If a file, simply deletes the file. If a directory, deletes the
    /// directory if it is empty.
    fn delete(&self) -> Result<(), Error> {
        unimplemented!()
    }
}
