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

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use sha2::{Digest, Sha256};

use serde::{de, Deserialize, Deserializer};

use super::traits::{Error, Resource, ResourceState};

use failure::ResultExt;
use util::PathExt;

/// A file resource. This can actually be a file *or* directory.
///
/// TODO: Split the directory portion out into a "glob" resource whose state
/// changes when the list of matched files changes.
#[derive(Serialize, Eq, Clone)]
pub struct FilePath {
    path: PathBuf,
}

impl FilePath {
    pub fn new(path: PathBuf) -> FilePath {
        FilePath {
            path: path.normalize(),
        }
    }

    /// Assumes this resource is a regular file and returns its checksum.
    fn file_state(&self) -> Result<ResourceState, Error> {
        let mut hasher = Sha256::default();

        let mut f = match fs::File::open(&self.path) {
            Ok(f) => Ok(f),
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    return Ok(ResourceState::Missing);
                }
                _ => Err(err),
            },
        }.with_context(|_| {
            format!("Could not open file {:?}", self.path)
        })?;

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
        FilePath::new(PathBuf::from(s.as_ref()))
    }
}

impl FromStr for FilePath {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(FilePath::new(PathBuf::from(s)))
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

impl<'de> Deserialize<'de> for FilePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
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
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    return Ok(());
                }
                _ => Err(err),
            },
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
            Ok(fs::remove_file(&self.path)?)
        }
    }
}

impl Hash for FilePath {
    #[cfg(windows)]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let iter = self
            .path
            .to_str()
            .unwrap_or("")
            .chars()
            .flat_map(char::to_lowercase);
        for c in iter {
            c.hash(state);
        }

        '\0'.hash(state);
    }

    #[cfg(unix)]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.path.hash(state)
    }
}

impl Ord for FilePath {
    #[cfg(windows)]
    fn cmp(&self, other: &FilePath) -> Ordering {
        // The Ord implementation for `std::path::Path` is case-sensitive. It's
        // important that path comparisons on Windows are case-insensitive. The
        // nodes in the build graph don't link up correctly when file paths only
        // differ by case. Thus, we implement our own path comparison here.
        let a = self
            .path
            .to_str()
            .unwrap_or("")
            .chars()
            .flat_map(char::to_lowercase);
        let mut b = other
            .path
            .to_str()
            .unwrap_or("")
            .chars()
            .flat_map(char::to_lowercase);
        a.cmp(&mut b)
    }

    #[cfg(unix)]
    fn cmp(&self, other: &FilePath) -> Ordering {
        // File paths are case sensitive on non-Windows platforms.
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for FilePath {
    fn partial_cmp(&self, other: &FilePath) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FilePath {
    fn eq(&self, other: &FilePath) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized() {
        assert_eq!(
            FilePath::new(PathBuf::from("./foo/..//bar/")),
            FilePath::new(PathBuf::from("bar"))
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_comparison() {
        assert_eq!(FilePath::from("foobar/baz"), FilePath::from("foobar/baz"));
        assert_ne!(FilePath::from("doobar/baz"), FilePath::from("foobar/baz"));

        assert!(FilePath::from("abc") < FilePath::from("abd"));
        assert!(FilePath::from("abc") < FilePath::from("abcd"));

        // Case insensitive comparison
        assert_eq!(FilePath::from("foobar/baz"), FilePath::from("FooBar/Baz"));
        assert_eq!(FilePath::from("foobar/baz"), FilePath::from(r"FooBar\Baz"));
        assert_ne!(FilePath::from("foobar/baz"), FilePath::from("FooBar/Bazz"));
        assert!(FilePath::from("foobar/baz") < FilePath::from("FooBar/Bazz"));
    }
}
