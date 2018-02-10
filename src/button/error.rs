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

use std::error;
use std::fmt;

use build_graph;
use rules;

/// The main error enum. All other errors should trickle down into this one. If
/// a build fails, this is what it should return.
#[derive(Debug)]
pub enum Error {
    /// An error reading or parsing rules.
    Rules(rules::Error),

    /// An error with the build graph.
    Graph(build_graph::Error),
}

impl From<rules::Error> for Error {
    fn from(err: rules::Error) -> Error {
        Error::Rules(err)
    }
}

impl From<build_graph::Error> for Error {
    fn from(err: build_graph::Error) -> Error {
        Error::Graph(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Rules(ref err) => write!(f, "{}", err),
            Error::Graph(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Rules(ref err) => err.description(),
            Error::Graph(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Rules(ref err) => Some(err),
            Error::Graph(ref err) => Some(err),
        }
    }
}
