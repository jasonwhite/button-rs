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
use std::ffi::OsStr;
use std::fmt::{self, Write as FmtWrite};
use std::io::{self, Write};
use std::iter;
use std::ops;

use tempfile::{NamedTempFile, TempPath};

/// Helper type for formatting command line arguments.
#[derive(Ord, Eq, PartialOrd, PartialEq, Hash, Debug)]
pub struct Arg(str);

impl Arg {
    pub fn new<S: AsRef<str> + ?Sized>(arg: &S) -> &Arg {
        unsafe { &*(arg.as_ref() as *const str as *const Arg) }
    }
}

impl ops::Deref for Arg {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Arg {
    /// Quotes the argument such that it is safe to pass to the shell.
    #[cfg(windows)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let quote =
            self.0.chars().any(|c| c == ' ' || c == '\t') || self.0.is_empty();

        if quote {
            f.write_char('"')?;
        }

        let mut backslashes: usize = 0;

        for x in self.0.chars() {
            if x == '\\' {
                backslashes += 1;
            } else {
                // Dump backslashes if we hit a quotation mark.
                if x == '"' {
                    // We need 2n+1 backslashes to escape a quote.
                    for _ in 0..(backslashes + 1) {
                        f.write_char('\\')?;
                    }
                }

                backslashes = 0;
            }

            f.write_char(x)?;
        }

        if quote {
            // Escape any trailing backslashes.
            for _ in 0..backslashes {
                f.write_char('\\')?;
            }

            f.write_char('"')?;
        }

        Ok(())
    }

    /// Quotes the argument such that it is safe to pass to the shell.
    #[cfg(unix)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let quote = self.0.chars().any(|c| " \n\t#<>'&|".contains(c))
            || self.0.is_empty();

        if quote {
            f.write_char('"')?;
        }

        for c in self.0.chars() {
            // Escape special characters.
            if "\\\"$~".contains(c) {
                f.write_char('\\')?;
            }

            f.write_char(c)?;
        }

        if quote {
            f.write_char('"')?;
        }

        Ok(())
    }
}

impl AsRef<Arg> for str {
    fn as_ref(&self) -> &Arg {
        Arg::new(self)
    }
}

impl AsRef<str> for Arg {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<OsStr> for Arg {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl AsRef<Arg> for String {
    fn as_ref(&self) -> &Arg {
        Arg::new(self)
    }
}

impl AsRef<Arg> for Arg {
    fn as_ref(&self) -> &Arg {
        self
    }
}

/// An owned `Arg`.
#[derive(
    Serialize, Deserialize, Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Debug,
)]
pub struct ArgBuf(String);

impl From<String> for ArgBuf {
    fn from(s: String) -> ArgBuf {
        ArgBuf(s)
    }
}

impl<'a> From<&'a Arg> for ArgBuf {
    fn from(s: &'a Arg) -> ArgBuf {
        ArgBuf::from(s.to_string())
    }
}

impl<'a> From<&'a str> for ArgBuf {
    fn from(s: &'a str) -> ArgBuf {
        ArgBuf::from(s.to_string())
    }
}

impl ops::Deref for ArgBuf {
    type Target = Arg;

    fn deref(&self) -> &Self::Target {
        &Arg::new(&self.0)
    }
}

impl AsRef<Arg> for ArgBuf {
    fn as_ref(&self) -> &Arg {
        self
    }
}

impl AsRef<OsStr> for ArgBuf {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl fmt::Display for ArgBuf {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, formatter)
    }
}

/// A list of arguments.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug,
)]
pub struct Arguments(Vec<ArgBuf>);

impl Default for Arguments {
    fn default() -> Arguments {
        Arguments(Vec::new())
    }
}

impl Arguments {
    pub fn new() -> Arguments {
        Arguments::default()
    }

    /// Counts the number of bytes that the list of arguments takes up when
    /// passed to the operating system.
    #[cfg(windows)]
    pub fn byte_count(&self) -> usize {
        use super::counter::Counter;
        use std::fmt::Write as FmtWrite;

        let mut counter = Counter::new();

        let mut iter = self.into_iter();

        if let Some(arg) = iter.next() {
            write!(counter, "{}", arg).unwrap();
        }

        for arg in iter {
            write!(counter, " {}", arg).unwrap();
        }

        // +1 for the final NULL terminator.
        counter += 1;

        counter.count()
    }

    #[cfg(unix)]
    pub fn byte_count(&self) -> usize {
        let mut size: usize = 0;

        for arg in self {
            // +1 for the NULL terminator.
            size += arg.len() + 1;
        }

        // +1 for the final NULL terminator.
        size += 1;

        size
    }

    /// Returns true if the argument list exceeds the operating system limits.
    ///
    /// Useful to know when generating a response file is appropriate.
    pub fn too_large(&self) -> bool {
        #[cfg(windows)]
        {
            self.byte_count() > 32768
        }

        #[cfg(unix)]
        {
            self.byte_count() > 0x20000
        }
    }

    /// Generates a temporary response file for the list of arguments.
    pub fn response_file(&self) -> io::Result<TempPath> {
        let tempfile = NamedTempFile::new()?;

        {
            let mut writer = io::BufWriter::new(&tempfile);

            // Write UTF-8 BOM. Some tools require this to properly decode it
            // as UTF-8 instead of ASCII.
            writer.write_all(b"\xEF\xBB\xBF")?;

            self.write_response_file(&mut writer)?;

            // Explicitly flush to catch any errors.
            writer.flush()?;
        }

        Ok(tempfile.into_temp_path())
    }

    /// Write a response file to an arbitrary writer.
    fn write_response_file(&self, writer: &mut io::Write) -> io::Result<()> {
        let mut iter = self.into_iter();

        if let Some(arg) = iter.next() {
            write!(writer, "{}", arg)?;
        }

        for arg in iter {
            write!(writer, " {}", arg)?;
        }

        // Some programs require a trailing new line (notably LIB.exe and
        // LINK.exe).
        writer.write_all(b"\n")?;

        Ok(())
    }
}

impl fmt::Display for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.iter();

        if let Some(arg) = iter.next() {
            write!(f, "{}", arg)?;
        }

        for arg in iter {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

impl From<Vec<ArgBuf>> for Arguments {
    fn from(args: Vec<ArgBuf>) -> Arguments {
        Arguments(args)
    }
}

impl iter::FromIterator<ArgBuf> for Arguments
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = ArgBuf>,
    {
        let mut args = Arguments::new();
        args.extend(iter);
        args
    }
}

impl iter::Extend<ArgBuf> for Arguments
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = ArgBuf>,
    {
        for a in iter {
            self.push(a)
        }
    }
}

impl ops::Deref for Arguments {
    type Target = Vec<ArgBuf>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Arguments {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a Arguments {
    type Item = &'a Arg;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self.0.iter(),
        }
    }
}

pub struct Iter<'a> {
    inner: ::std::slice::Iter<'a, ArgBuf>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Arg;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(ArgBuf::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_arg_display() {
        assert_eq!(format!("{}", Arg::new("foo bar")), "\"foo bar\"");
        assert_eq!(format!("{}", Arg::new("foo\tbar")), "\"foo\tbar\"");
        assert_eq!(format!("{}", Arg::new("foobar")), "foobar");
        assert_eq!(
            format!("{}", Arg::new("\"foo bar\"")),
            "\"\\\"foo bar\\\"\""
        );
        assert_eq!(format!("{}", Arg::new(r"C:\foo\bar")), r"C:\foo\bar");
        assert_eq!(format!("{}", Arg::new(r"\\foo\bar")), r"\\foo\bar");
    }

    #[test]
    #[cfg(unix)]
    fn test_arg_display() {
        assert_eq!(format!("{}", Arg::new("foo bar")), "\"foo bar\"");
        assert_eq!(format!("{}", Arg::new("foo\tbar")), "\"foo\tbar\"");
        assert_eq!(format!("{}", Arg::new("foo\nbar")), "\"foo\nbar\"");
        assert_eq!(format!("{}", Arg::new("foobar")), "foobar");
        assert_eq!(
            format!("{}", Arg::new("\"foo bar\"")),
            "\"\\\"foo bar\\\"\""
        );
        assert_eq!(format!("{}", Arg::new(r"\\foo\bar")), r"\\\\foo\\bar");
        assert_eq!(format!("{}", Arg::new(r"$HOME")), r"\$HOME");
        assert_eq!(format!("{}", Arg::new(r"foo>bar")), "\"foo>bar\"");
        assert_eq!(format!("{}", Arg::new(r"foo&bar")), "\"foo&bar\"");
        assert_eq!(format!("{}", Arg::new(r"~")), r"\~");
        assert_eq!(format!("{}", Arg::new(r"foo|bar")), "\"foo|bar\"");

        assert_eq!(format!("{}", ArgBuf::from(r"foo bar")), "\"foo bar\"");
    }
}
