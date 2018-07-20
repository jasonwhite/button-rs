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

mod build;
mod clean;
mod graph;

pub use self::build::Build;
pub use self::clean::Clean;
pub use self::graph::Graph;

use failure::Error;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Builds everything.
    #[structopt(name = "build")]
    Build(Build),

    /// Deletes all files created during the build.
    #[structopt(name = "clean")]
    Clean(Clean),

    /// Generates a graphviz file of the build graph.
    #[structopt(name = "graph")]
    Graph(Graph),
}

impl Command {
    pub fn main(&self) -> Result<(), Error> {
        match *self {
            Command::Build(ref x) => x.main(),
            Command::Clean(ref x) => x.main(),
            Command::Graph(ref x) => x.main(),
        }
    }
}
