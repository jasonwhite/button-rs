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

use clap::{ArgMatches, Result};

pub use self::build::Build;
pub use self::clean::Clean;
pub use self::graph::Graph;

pub enum Command {
    Build(Build),
    Clean(Clean),
    Graph(Graph),
}

impl Command {
    pub fn from_matches<'a>(name: &str,
                            matches: &ArgMatches<'a>)
                            -> Result<Command> {
        match name {
            "build" => Build::from_matches(matches).map(Command::Build),
            "clean" => Clean::from_matches(matches).map(Command::Clean),
            "graph" => Graph::from_matches(matches).map(Command::Graph),

            // If all subcommands are matched above, then this shouldn't happen.
            _ => unreachable!(),
        }
    }

    pub fn run(&self) -> i32 {
        match self {
            &Command::Build(ref x) => x.run(),
            &Command::Clean(ref x) => x.run(),
            &Command::Graph(ref x) => x.run(),
        }
    }
}
