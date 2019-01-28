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

use std::fmt::{self, Debug, Display};
use std::path::PathBuf;

use failure::{Backtrace, Context};
use serde::{Deserialize, Serialize};

use crate::graph::NodeIndex;

pub use failure::{Error, Fail, ResultExt};

/// A serializable error.
///
/// The cause chain is stored as a vector of strings such that it can be
/// serialized and deserialized easily.
///
/// In practice, we don't care about retaining type information because we only
/// care about displaying the error chain to the user as a string.
#[derive(Serialize, Deserialize)]
pub struct SerError(Vec<String>);

impl<'a> From<&'a Error> for SerError {
    fn from(error: &Error) -> SerError {
        SerError(error.iter_chain().map(|c| format!("{}", c)).collect())
    }
}

impl<'a> From<&'a BuildError> for SerError {
    fn from(error: &BuildError) -> SerError {
        let mut errors = Vec::new();
        errors.push(format!("{}", error));

        let mut error = error.cause();

        while let Some(cause) = error {
            errors.push(format!("{}", cause));
            error = cause.cause();
        }

        SerError(errors)
    }
}

impl Debug for SerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0[0])
    }
}

impl Display for SerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0[0])
    }
}

impl std::error::Error for SerError {}

#[derive(Fail, Debug)]
pub struct InvalidEdges(pub Vec<(NodeIndex, NodeIndex)>);

impl Display for InvalidEdges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} detected edge(s) would have changed build order",
            self.0.len()
        )
    }
}

#[derive(Debug)]
pub struct ChecksumError(Error);

#[derive(Debug)]
pub struct DeletionError(Error);

#[derive(Debug)]
pub struct EdgeError(NodeIndex, NodeIndex);

/// An error that can occur during a build.
#[derive(Fail, Debug)]
pub enum ErrorKind {
    /// A failure parsing rules.
    ParsingRules,

    /// A failure reading the build state.
    LoadState(PathBuf),

    /// A failure writing the build state.
    SaveState(PathBuf),

    /// A failure deleting the build state.
    CleanState(PathBuf),

    /// A failure synchronizing the build state.
    SyncState,

    /// Detected edges that would have caused the build order to change.
    InvalidEdges(Vec<(String, String)>),

    /// One or more failed tasks.
    TaskErrors(Vec<(NodeIndex, Error)>),

    /// One or more failed deletions.
    DeleteErrors(Vec<(NodeIndex, Error)>),

    /// Failed creating build graph.
    BuildGraph,

    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::ParsingRules => write!(f, "Could not parse build rules"),
            ErrorKind::LoadState(path) => write!(
                f,
                "Failed loading build state from '{}'",
                path.display()
            ),
            ErrorKind::SaveState(path) => {
                write!(f, "Failed saving build state to '{}'", path.display())
            }
            ErrorKind::CleanState(path) => {
                write!(f, "Failed deleting build state '{}'", path.display())
            }
            ErrorKind::SyncState => write!(f, "Failed updating build graph"),
            ErrorKind::InvalidEdges(edges) => {
                writeln!(
                    f,
                    "{} detected edge(s) would have changed build order:",
                    edges.len()
                )?;

                for (from, to) in edges {
                    writeln!(f, "- {} -> {}", from, to)?;
                }

                Ok(())
            }
            ErrorKind::TaskErrors(errors) => {
                write!(f, "{} task(s) failed", errors.len())
            }
            ErrorKind::DeleteErrors(errors) => {
                write!(f, "{} resources(s) could not be deleted", errors.len())
            }
            ErrorKind::BuildGraph => write!(f, "Failed creating build graph"),
            ErrorKind::Other(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub struct BuildError {
    inner: Context<ErrorKind>,
}

impl BuildError {
    pub fn kind(&self) -> &ErrorKind {
        &*self.inner.get_context()
    }
}

impl Fail for BuildError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
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

impl From<Error> for BuildError {
    fn from(err: Error) -> Self {
        BuildError {
            inner: Context::new(ErrorKind::Other(err.into())),
        }
    }
}

impl From<SerError> for BuildError {
    fn from(err: SerError) -> BuildError {
        let mut causes = err.0.into_iter().rev();

        let mut err = Error::from_boxed_compat(causes.next().unwrap().into());

        for cause in causes {
            err = err.context(cause).into();
        }

        ErrorKind::Other(err.into()).into()
    }
}
