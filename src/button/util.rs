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
use std::ops;
use std::path::{Component, Path, PathBuf};

/// A tri-state for checking if we should do things.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Copy, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum NeverAlwaysAuto {
    /// Never do the thing.
    Never,

    /// Always do the thing.
    Always,

    /// Only do the thing under certain circumstances.
    Auto,
}

impl Default for NeverAlwaysAuto {
    /// Never do the thing by default.
    fn default() -> Self {
        NeverAlwaysAuto::Never
    }
}

/// A fake writer to count the number of items going into it.
#[allow(dead_code)]
pub struct Counter {
    count: usize,
}

impl Counter {
    #[allow(dead_code)]
    pub fn new() -> Counter {
        Counter { count: 0 }
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.count
    }
}

impl ops::AddAssign<usize> for Counter {
    fn add_assign(&mut self, rhs: usize) {
        self.count += rhs;
    }
}

impl fmt::Write for Counter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.count += s.len();
        Ok(())
    }
}

pub trait PathExt {
    /// Returns a normalized path. This does not touch the file system at all.
    fn normalize(&self) -> PathBuf;
}

impl PathExt for Path {
    #[cfg(windows)]
    fn normalize(&self) -> PathBuf {
        use std::ffi::OsString;
        use std::path::Prefix;

        let mut new_path = PathBuf::new();

        let mut components = self.components();

        if self.as_os_str().len() >= 260 {
            // If the path is >= 260 characters, we should prefix it with
            // '\\?\' if possible.
            if let Some(c) = components.next() {
                match c {
                    Component::CurDir => {}
                    Component::RootDir
                    | Component::ParentDir
                    | Component::Normal(_) => {
                        // Can't add the prefix. It's a relative path.
                        new_path.push(c.as_os_str());
                    }
                    Component::Prefix(prefix) => {
                        match prefix.kind() {
                            Prefix::UNC(server, share) => {
                                let mut p = OsString::from(r"\\?\UNC\");
                                p.push(server);
                                p.push(r"\");
                                p.push(share);
                                new_path.push(p);
                            }
                            Prefix::Disk(_) => {
                                let mut p = OsString::from(r"\\?\");
                                p.push(c.as_os_str());
                                new_path.push(p);
                            }
                            _ => {
                                new_path.push(c.as_os_str());
                            }
                        };
                    }
                };
            }
        }

        for c in components {
            match c {
                Component::CurDir => {}
                Component::ParentDir => {
                    let pop = match new_path.components().next_back() {
                        Some(Component::Prefix(_))
                        | Some(Component::RootDir) => true,
                        Some(Component::Normal(s)) => !s.is_empty(),
                        _ => false,
                    };

                    if pop {
                        new_path.pop();
                    } else {
                        new_path.push("..");
                    }
                }
                _ => {
                    new_path.push(c.as_os_str());
                }
            };
        }

        if new_path.as_os_str().is_empty() {
            new_path.push(".");
        }

        new_path
    }

    #[cfg(unix)]
    fn normalize(&self) -> PathBuf {
        let mut new_path = PathBuf::new();

        for c in self.components() {
            match c {
                Component::CurDir => {}
                Component::ParentDir => {
                    let pop = match new_path.components().next_back() {
                        Some(Component::Prefix(_))
                        | Some(Component::RootDir) => true,
                        Some(Component::Normal(s)) => !s.is_empty(),
                        _ => false,
                    };

                    if pop {
                        new_path.pop();
                    } else {
                        new_path.push("..");
                    }
                }
                _ => {
                    new_path.push(c.as_os_str());
                }
            };
        }

        if new_path.as_os_str().is_empty() {
            new_path.push(".");
        }

        new_path
    }
}

/// Check if an iterator is empty or if the predicate returns true for any item.
pub fn empty_or_any<I, F>(iter: &mut I, mut f: F) -> bool
where
    I: Iterator,
    F: FnMut(I::Item) -> bool,
{
    match iter.next() {
        None => return true, // Empty
        Some(x) => {
            if f(x) {
                return true;
            }
        }
    };

    for x in iter {
        if f(x) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_norm() {
        assert_eq!(Path::new("../foo").parent(), Some(Path::new("..")));
        assert_eq!(Path::new("foo").normalize(), Path::new("foo"));
        assert_eq!(Path::new("./foo").normalize(), Path::new("foo"));
        assert_eq!(Path::new(".").normalize(), Path::new("."));
        assert_eq!(Path::new("..").normalize(), Path::new(".."));
        assert_eq!(Path::new(r"..\..").normalize(), Path::new(r"..\.."));
        assert_eq!(Path::new(r"..\..\..").normalize(), Path::new(r"..\..\.."));
        assert_eq!(Path::new("").normalize(), Path::new("."));
        assert_eq!(Path::new("foo/bar").normalize(), Path::new(r"foo\bar"));
        assert_eq!(
            Path::new("C:/foo/../bar").normalize(),
            Path::new(r"C:\bar")
        );
        assert_eq!(Path::new("C:/../bar").normalize(), Path::new(r"C:\bar"));
        assert_eq!(Path::new("C:/../../bar").normalize(), Path::new(r"C:\bar"));
        assert_eq!(Path::new("foo//bar///").normalize(), Path::new(r"foo\bar"));
        assert_eq!(
            Path::new(r"\\server\share\..\foo").normalize(),
            Path::new(r"\\server\share\foo")
        );
        assert_eq!(
            Path::new(r"\\server\share\..\foo\..").normalize(),
            Path::new(r"\\server\share")
        );
        assert_eq!(
            Path::new(r"..\foo\..\..\bar").normalize(),
            Path::new(r"..\..\bar")
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_norm() {
        assert_eq!(Path::new("../foo").parent(), Some(Path::new("..")));
        assert_eq!(Path::new("foo").normalize(), Path::new("foo"));
        assert_eq!(Path::new("./foo").normalize(), Path::new("foo"));
        assert_eq!(Path::new(".").normalize(), Path::new("."));
        assert_eq!(Path::new("..").normalize(), Path::new(".."));
        assert_eq!(Path::new("../..").normalize(), Path::new("../.."));
        assert_eq!(Path::new("../../..").normalize(), Path::new("../../.."));
        assert_eq!(Path::new("").normalize(), Path::new("."));
        assert_eq!(Path::new("foo/bar").normalize(), Path::new("foo/bar"));
        assert_eq!(Path::new("/foo/../bar").normalize(), Path::new("/bar"));
        assert_eq!(Path::new("/../bar").normalize(), Path::new("/bar"));
        assert_eq!(Path::new("/../../bar").normalize(), Path::new("/bar"));
        assert_eq!(Path::new("foo//bar///").normalize(), Path::new("foo/bar"));
        assert_eq!(
            Path::new("../foo/../../bar").normalize(),
            Path::new("../../bar")
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_norm_long_paths() {
        use std::iter;

        let long_name: String = iter::repeat('a').take(260).collect();
        let long_name = long_name.as_str();

        // Long paths
        assert_eq!(
            PathBuf::from(String::from(r"C:\") + long_name).normalize(),
            PathBuf::from(String::from(r"\\?\C:\") + long_name)
        );
        assert_eq!(
            PathBuf::from(String::from(r"\\server\share\") + long_name)
                .normalize(),
            PathBuf::from(String::from(r"\\?\UNC\server\share\") + long_name)
        );

        // Long relative paths
        assert_eq!(
            PathBuf::from(String::from(r"..\relative\") + long_name)
                .normalize(),
            PathBuf::from(String::from(r"..\relative\") + long_name)
        );
        assert_eq!(
            PathBuf::from(String::from(r".\relative\") + long_name).normalize(),
            PathBuf::from(String::from(r"relative\") + long_name)
        );
    }

    #[test]
    fn test_empty_or_any() {
        assert!(empty_or_any(&mut [].iter(), |x: &bool| !x));
        assert!(empty_or_any(&mut [true, false, true].iter(), |x| !x));
    }
}
