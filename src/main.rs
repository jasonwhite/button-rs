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
#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate petgraph;
extern crate num_cpus;
extern crate crossbeam;
extern crate sha2;
extern crate generic_array;
#[macro_use]
extern crate error_chain;

mod cli;
mod rules;
mod build;
mod error;
mod node;
mod resource;
mod task;
mod graph;
mod retry;

use std::process::exit;

fn main() {

    let app_matches = cli::app().get_matches();

    let (name, matches) = app_matches.subcommand();

    if let Some(matches) = matches {
        exit(match cli::Command::from_matches(name, matches) {
                 Ok(command) => command.run(),
                 Err(err) => {
                     println!("{}", err);
                     1
                 }
             });
    }
}
