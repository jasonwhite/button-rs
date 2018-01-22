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
use std::io::Read;

use sha2::{Sha256, Digest};

use serde::{de, Deserialize, Deserializer};

use super::traits::{Resource, ResourceState, Error};


/// A file resource. This can actually be a file *or* directory.
#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct FilePath {
    path: PathBuf,
}

impl FilePath {
    #[allow(dead_code)]
    fn new(path: PathBuf) -> FilePath {
        FilePath { path: path }
    }

    /// Assumes this resource is a regular file and returns its checksum.
    fn file_state(&self) -> Result<ResourceState, Error> {
        let mut hasher = Sha256::default();

        let mut f = match fs::File::open(&self.path) {
            Ok(f) => Ok(f),
            Err(err) => {
                match err.kind() {
                    io::ErrorKind::NotFound => {
                        return Ok(ResourceState::Missing);
                    }
                    _ => Err(err),
                }
            }
        }?;

        const BUF_SIZE: usize = 16384;

        let mut buf = [0u8; BUF_SIZE];

        loop {
            let n = f.read(&mut buf)?;

            if n == 0 || n < BUF_SIZE {
                break;
            }

            hasher.input(&buf[0..n]);
        }

        Ok(ResourceState::Checksum(hasher.result()))
    }

    /// Assumes this resource is a directory and returns the checksum of its
    /// file contents.
    fn dir_state(&self) -> Result<ResourceState, Error> {
        let mut hasher = Sha256::default();

        let mut names = vec![];

        for entry in fs::read_dir(&self.path)? {
            names.push(entry?.file_name());
        }

        // The order in which files are listed is not guaranteed to be sorted.
        // Whether or not it is sorted depends on the file system
        // implementation. Thus, we sort them to eliminate that potential
        // source of non-determinism.
        names.sort();

        for name in names {
            if let Some(name) = name.to_str() {
                hasher.input(name.as_bytes());
            }
        }

        Ok(ResourceState::Checksum(hasher.result()))
    }
}

impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for FilePath {
    fn from(s: &'a T) -> FilePath {
        FilePath { path: PathBuf::from(s.as_ref()) }
    }
}

impl FromStr for FilePath {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(FilePath { path: PathBuf::from(s) })
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl fmt::Debug for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

// Derserialize a `FilePath` from a string.
impl<'de> Deserialize<'de> for FilePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl Resource for FilePath {
    /// If a file, the checksum is of the contents of the file. If a directory,
    /// the checksum is of the sorted list of directory entries. Thus, if a file
    /// is added or removed from a directory, the checksum changes.
    fn state(&self) -> Result<ResourceState, Error> {
        if self.path.is_dir() {
            self.dir_state()
        } else {
            // Assume its a file even if its not. It'll error out if there are
            // problems reading it.
            self.file_state()
        }
    }

    /// If a file, simply deletes the file. If a directory, deletes the
    /// directory if it is empty.
    fn delete(&self) -> Result<(), Error> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => Ok(metadata),
            Err(err) => {
                match err.kind() {
                    io::ErrorKind::NotFound => {
                        return Ok(());
                    }
                    _ => Err(err),
                }
            }
        }?;

        if metadata.is_dir() {
            // Ignore errors for directory deletion. It's not usually a problem
            // if a directory fails to get deleted. Directory deletion can fail
            // for a number of reasons:
            //  - A user may have created a file inside of it.
            //  - An untracked output may have been created inside of it.
            //  - On Windows, someone may have a lock on the directory.
            let _ = fs::remove_dir(&self.path);
            Ok(())
        } else {
            // Assume its a file even if its not. It'll error out if there are
            // problems deleting it.
            fs::remove_file(&self.path)
        }
    }
}
