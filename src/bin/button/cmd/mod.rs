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
mod dump;
mod graph;
mod replay;

pub use self::build::Build;
pub use self::clean::Clean;
pub use self::dump::Dump;
pub use self::graph::Graph;
pub use self::replay::Replay;

use structopt::StructOpt;

use button::Error;

use crate::opts::GlobalOpts;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Builds everything.
    #[structopt(name = "build")]
    Build(Build),

    /// Deletes all files created during the build.
    #[structopt(name = "clean")]
    Clean(Clean),

    /// Dumps the build graph.
    #[structopt(name = "dump")]
    Dump(Dump),

    /// Generates a graphviz file of the build graph.
    #[structopt(name = "graph")]
    Graph(Graph),

    /// Replays a build log file.
    #[structopt(name = "replay")]
    Replay(Replay),
}

impl Command {
    pub fn main(self, global: &GlobalOpts) -> Result<(), Error> {
        match self {
            Command::Build(x) => x.main(global),
            Command::Clean(x) => x.main(global),
            Command::Dump(x) => x.main(global),
            Command::Graph(x) => x.main(global),
            Command::Replay(x) => x.main(global),
        }
    }
}
