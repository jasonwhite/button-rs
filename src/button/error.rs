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

use rules;
use graph;

/// The main error enum. All other errors should trickle down into this one. If
/// a build fails, this is what it should return.
#[derive(Debug)]
pub enum Error<'a> {
    /// An error reading or parsing rules.
    Rules(rules::Error),

    /// An error with the build graph.
    Graph(graph::Error<'a>),
}

impl<'a> From<rules::Error> for Error<'a> {
    fn from(err: rules::Error) -> Error<'a> {
        Error::Rules(err)
    }
}

impl<'a> From<graph::Error<'a>> for Error<'a> {
    fn from(err: graph::Error<'a>) -> Error<'a> {
        Error::Graph(err)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Rules(ref err) => write!(f, "{}", err),
            Error::Graph(ref err) => write!(f, "{}", err),
        }
    }
}

impl<'a> error::Error for Error<'a> {
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