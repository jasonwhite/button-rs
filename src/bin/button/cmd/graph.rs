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
use std::path::PathBuf;

use opts::Edges;

#[derive(StructOpt, Debug)]
pub struct Graph {
    /// Path to the build description. If not specified, finds "button.json"
    /// in the current directory or parent directories.
    #[structopt(long = "file", short = "f", parse(from_os_str))]
    file: Option<PathBuf>,

    /// Only display the subgraph that will be traversed on an update. This
    /// has to query the file system for changes to resources.
    #[structopt(long = "changes")]
    changes: bool,

    /// Display the cached graph from the previous build.
    #[structopt(long = "cached")]
    cached: bool,

    /// Display the full name for each node instead of the abbreviated name.
    #[structopt(long = "full")]
    full: bool,

    /// Type of edges to show in the graph.
    #[structopt(
        long = "edges",
        default_value = "explicit",
        raw(
            possible_values = "&Edges::variants()",
            case_insensitive = "true"
        )
    )]
    edges: Edges,
}

impl Graph {
    /// Shows a pretty graph of the build.
    pub fn main(&self) -> i32 {
        println!("{:#?}", self);

        0
    }
}
