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

use std::fmt::{self, Display};
use std::io;

use bincode;
use failure::{Backtrace, Context, Fail};
use serde_json;
use tempfile;

use graph::NodeIndex;

pub use failure::{Error, ResultExt};

/// A serializable error.
///
/// The cause chain is stored as a vector of strings such that it can be
/// serialized and deserialized easily.
///
/// In practice, we don't care about retaining type information because we only
/// care about displaying the error chain to the user as a string.
#[derive(Serialize, Deserialize)]
pub struct SerError(Vec<String>);

impl SerError {
    pub fn new(error: &Error) -> SerError {
        SerError(error.iter_chain().map(|c| format!("{}", c)).collect())
    }
}

impl fmt::Debug for SerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.0[0])
    }
}

impl Into<Error> for SerError {
    fn into(self) -> Error {
        let mut causes = self.0.into_iter().rev();

        let mut err = Error::from_boxed_compat(causes.next().unwrap().into());

        for cause in causes {
            err = err.context(cause).into();
        }

        err
    }
}

#[derive(Debug)]
pub struct TaskError(Error);

#[derive(Debug)]
pub struct ChecksumError(Error);

#[derive(Debug)]
pub struct DeletionError(Error);

#[derive(Debug)]
pub struct EdgeError(NodeIndex, NodeIndex);

/// An error that can occur during a build.
#[derive(Fail, Debug)]
pub enum ErrorKind {
    /// An io error.
    #[fail(display = "{}", _0)]
    Io(io::Error),

    /// A JSON parsing error.
    #[fail(display = "{}", _0)]
    Json(serde_json::error::Error),

    /// A bincode error.
    #[fail(display = "{}", _0)]
    Bincode(bincode::Error),

    /// A tempfile error.
    #[fail(display = "{}", _0)]
    TempFile(tempfile::PersistError),

    /// One or more tasks did not complete successfully.
    #[fail(display = "One or more tasks failed")]
    TaskFailed(Vec<(NodeIndex, TaskError)>),

    /// Failed to calculate the state of one or more resources.
    #[fail(display = "Failed calculating checksums")]
    Checksum(Vec<(NodeIndex, ChecksumError)>),

    /// One or more resources were not deleted successfully.
    #[fail(display = "One or more resources could not be deleted")]
    ResourceDeletion(Vec<(NodeIndex, DeletionError)>),

    /// An edge would change the build order. That is, a new dependency tried
    /// to be added to a non-root resource.
    #[fail(
        display = "One or more implicit dependencies would change build order"
    )]
    InvalidEdge(Vec<EdgeError>),

    /// A failure to parse rules.
    #[fail(display = "Failed parsing build rules")]
    BuildRules,

    /// A plain old string. Useful for attaching a context to one of these
    /// errors.
    #[fail(display = "{}", _0)]
    Custom(String),
}

#[derive(Debug)]
pub struct BuildError {
    inner: Context<ErrorKind>,
}

impl Fail for BuildError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl BuildError {
    pub fn kind(&self) -> &ErrorKind {
        &*self.inner.get_context()
    }
}

impl From<ErrorKind> for BuildError {
    fn from(kind: ErrorKind) -> Self {
        BuildError {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for BuildError {
    fn from(inner: Context<ErrorKind>) -> Self {
        BuildError { inner }
    }
}

impl<'a> From<Context<&'a str>> for BuildError {
    fn from(inner: Context<&'a str>) -> Self {
        BuildError {
            inner: inner.map(|x| ErrorKind::Custom(x.to_string())),
        }
    }
}

impl<'a> From<Context<String>> for BuildError {
    fn from(inner: Context<String>) -> Self {
        BuildError {
            inner: inner.map(|x| ErrorKind::Custom(x)),
        }
    }
}

impl From<io::Error> for BuildError {
    fn from(err: io::Error) -> Self {
        BuildError {
            inner: Context::new(ErrorKind::Io(err)),
        }
    }
}

impl From<serde_json::error::Error> for BuildError {
    fn from(err: serde_json::error::Error) -> Self {
        BuildError {
            inner: Context::new(ErrorKind::Json(err)),
        }
    }
}

impl From<bincode::Error> for BuildError {
    fn from(err: bincode::Error) -> Self {
        BuildError {
            inner: Context::new(ErrorKind::Bincode(err)),
        }
    }
}

impl From<tempfile::PersistError> for BuildError {
    fn from(err: tempfile::PersistError) -> Self {
        BuildError {
            inner: Context::new(ErrorKind::TempFile(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let result: Result<(), BuildError> =
            Err(ErrorKind::TaskFailed(Vec::new()).into());
        let result: Result<(), BuildError> = result
            .with_context(|_| String::from("Test"))
            .map_err(|e| e.into());

        if let Err(err) = result {
            assert_eq!(format!("{}", err), "Test");
            assert_eq!(
                err.cause().map(|x| format!("{}", x)),
                Some(String::from("One or more tasks failed"))
            );
        }
    }
}
