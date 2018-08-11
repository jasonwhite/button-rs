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
