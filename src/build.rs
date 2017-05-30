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

use std::path::Path;

use error;
use graph;
use rules::Rules;

/// Represents a build. This holds the context necessary for all build
/// operations.
pub struct Build<'a> {

    /// Root of the build. This is the directory containing the "button.json"
    /// file and is the default path from which all subprocesses are spawned.
    /// The working directories of tasks are relative to this path.
    root: &'a Path,

    /// Whether or not this is a dry run. This needs to be passed to child build
    /// systems.
    dryrun: bool,

    // The thread pool. This needs to be passed to child build systems.
    //pool: &ThreadPool,

    // The event handler. Child build system event handlers should forward
    // their events to this handler.
    //events: &BuildEvents,
}

impl<'a> Build<'a> {
    pub fn new(root: &'a Path, dryrun: bool) -> Build {
        Build { root: root, dryrun: dryrun }
    }

    /// Runs a build.
    pub fn build<'b>(&self, rules: &'b Rules) -> Result<(), error::Error<'b>> {

        println!("Root directory: {:?}", self.root);

        if self.dryrun {
            println!("Note: This is a dry run. Nothing is affected.");
        }

        graph::from_rules(&rules)?;

        Ok(())
    }
}
