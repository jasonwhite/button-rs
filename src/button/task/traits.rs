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
use std::io;
use std::hash::Hash;

use serde::Serialize;

use res;

/// FIXME: Use a more abstract error type.
pub type Error = io::Error;

/// A task is a routine to be executed that produces resources as outputs.
///
/// Most tasks will be of the `Command` type. That is, the execution of a
/// process with arguments.
///
/// Since a task is anything that can be executed, we can have other built-in
/// tasks to aid with cross-platform compatibility. For example:
///  * Copying a file or directory.
///  * Downloading a file.
///  * Creating a directory.
pub trait Task
    : Serialize + Ord + PartialOrd + Eq + PartialEq + Hash + fmt::Display {
    /// Executes the task. The result of a task are the resources it used and
    /// the resources it output. These are its *implicit* inputs and outputs.
    /// Ideally, the *explicit* inputs and outputs are a subset of the
    /// *implicit* inputs and outputs.
    fn execute(&self, log: &mut io::Write) -> Result<(), Error>;

    /// Inputs the task knows about *a priori*. It must calculate these by
    /// *only* looking at the task parameters. It cannot do anything fancy like
    /// running an external process to determine these.
    fn known_inputs(&self) -> Vec<res::Any> {
        Vec::new()
    }

    /// Outputs the task knows about *a priori*. It must calculate these by
    /// *only* looking at the task parameters. It cannot do anything fancy like
    /// running an external process to determine these.
    fn known_outputs(&self) -> Vec<res::Any> {
        Vec::new()
    }
}
