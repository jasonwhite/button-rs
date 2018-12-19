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
use std::path::{Path, PathBuf};
use std::str::FromStr;

use sha2::{Digest, Sha256};

use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

use super::traits::{Resource, ResourceState};

use crate::error::{Error, ResultExt};
use crate::util::{self, PathExt};

/// A file resource. This can actually be a file *or* directory.
///
/// TODO: Split the directory portion out into a "glob" resource whose state
/// changes when the list of matched files changes.
#[derive(Eq, Clone)]
pub struct File {
    path: PathBuf,
}

impl File {
    pub fn new<P: AsRef<Path>>(path: P) -> File {
        File {
            path: path.as_ref().normalize(),
        }
    }

    /// Assumes this resource is a regular file and returns its checksum.
    fn file_state(&self, root: &Path) -> Result<ResourceState, Error> {
        let path = root.join(&self.path);
        let f = match fs::File::open(&path) {
            Ok(f) => Ok(f),
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    return Ok(ResourceState::Missing);
                }
                _ => Err(err),
            },
        }
        .with_context(|_| format!("Could not open file {:?}", self.path))?;

        Ok(ResourceState::Checksum(util::Sha256::from_reader(f)?))
    }

    /// Assumes this resource is a directory and returns the checksum of its
    /// file contents.
    fn dir_state(&self, root: &Path) -> Result<ResourceState, Error> {
        let path = root.join(&self.path);

        let mut hasher = Sha256::default();

        let mut names = Vec::new();

        for entry in fs::read_dir(&path)? {
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

        Ok(ResourceState::Checksum(hasher.result().into()))
    }
}

impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for File {
    fn from(s: &'a T) -> File {
        File::new(&Path::new(s.as_ref()))
    }
}

impl FromStr for File {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(File::new(&Path::new(s)))
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl Serialize for File {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.path.to_str() {
            Some(s) => serializer.serialize_str(s),
            None => Err(ser::Error::custom(
                "path contains invalid UTF-8 characters",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for File {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl Resource for File {
    /// If a file, the checksum is of the contents of the file. If a directory,
    /// the checksum is of the sorted list of directory entries. Thus, if a file
    /// is added or removed from a directory, the checksum changes.
    fn state(&self, root: &Path) -> Result<ResourceState, Error> {
        if self.path.is_dir() {
            self.dir_state(root)
        } else {
            // Assume its a file even if its not. It'll error out if there are
            // problems reading it.
            self.file_state(root)
        }
    }

    /// If a file, simply deletes the file. If a directory, deletes the
    /// directory if it is empty.
    fn delete(&self, root: &Path) -> Result<(), Error> {
        let path = root.join(&self.path);

        let metadata = match fs::metadata(&path) {
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
            let _ = fs::remove_dir(&path);
            Ok(())
        } else {
            // Assume its a file even if its not. It'll error out if there are
            // problems deleting it.
            fs::remove_file(&path)?;
            Ok(())
        }
    }
}

impl Hash for File {
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

impl Ord for File {
    #[cfg(windows)]
    fn cmp(&self, other: &File) -> Ordering {
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
    fn cmp(&self, other: &File) -> Ordering {
        // File paths are case sensitive on non-Windows platforms.
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &File) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for File {
    fn eq(&self, other: &File) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized() {
        assert_eq!(
            File::new(&Path::new("./foo/..//bar/")),
            File::new(&Path::new("bar"))
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_comparison() {
        assert_eq!(File::from("foobar/baz"), File::from("foobar/baz"));
        assert_ne!(File::from("doobar/baz"), File::from("foobar/baz"));

        assert!(File::from("abc") < File::from("abd"));
        assert!(File::from("abc") < File::from("abcd"));

        // Case insensitive comparison
        assert_eq!(File::from("foobar/baz"), File::from("FooBar/Baz"));
        assert_eq!(File::from("foobar/baz"), File::from(r"FooBar\Baz"));
        assert_ne!(File::from("foobar/baz"), File::from("FooBar/Bazz"));
        assert!(File::from("foobar/baz") < File::from("FooBar/Bazz"));
    }
}
