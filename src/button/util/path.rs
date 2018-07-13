// Copyright (c) 2018 Jason White
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

use std::path::{Component, Path, PathBuf};

pub trait PathExt {
    /// Returns a normalized path. This does not touch the file system at all.
    fn normalize(&self) -> PathBuf;

    /// Returns a path relative to the given base path.
    fn relative_from(&self, base: &Path) -> Option<PathBuf>;
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

    fn relative_from(&self, base: &Path) -> Option<PathBuf> {
        if self.is_absolute() != base.is_absolute() {
            if self.is_absolute() {
                Some(PathBuf::from(self))
            } else {
                None
            }
        } else {
            let mut ita = self.components();
            let mut itb = base.components();
            let mut comps: Vec<Component> = vec![];
            loop {
                match (ita.next(), itb.next()) {
                    (None, None) => break,
                    (Some(a), None) => {
                        comps.push(a);
                        comps.extend(ita.by_ref());
                        break;
                    }
                    (None, _) => comps.push(Component::ParentDir),
                    (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                    (Some(a), Some(b)) if b == Component::CurDir => {
                        comps.push(a)
                    }
                    (Some(_), Some(b)) if b == Component::ParentDir => {
                        return None
                    }
                    (Some(a), Some(_)) => {
                        comps.push(Component::ParentDir);
                        for _ in itb {
                            comps.push(Component::ParentDir);
                        }
                        comps.push(a);
                        comps.extend(ita.by_ref());
                        break;
                    }
                }
            }
            Some(comps.iter().map(|c| c.as_os_str()).collect())
        }
    }
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
    #[cfg(unix)]
    fn test_relative_from() {
        assert_eq!(
            Path::new("/bar/foo").relative_from(Path::new("/bar")),
            Some(PathBuf::from("foo"))
        );
        assert_eq!(
            Path::new("/foo").relative_from(Path::new("/bar")),
            Some(PathBuf::from("../foo"))
        );
        assert_eq!(
            Path::new("/foo/bar").relative_from(Path::new("/foo/bar")),
            Some(PathBuf::from(""))
        );
        assert_eq!(
            Path::new("foobar").relative_from(Path::new("foobar")),
            Some(PathBuf::from(""))
        );
    }
}
