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
#[macro_use] extern crate clap;

mod cli;

fn main() {

    let app_matches = cli::app().get_matches();

    let (name, matches) = app_matches.subcommand();

    if let Some(matches) = matches {
        match cli::subcommand(name, matches) {
            Ok(cli::Command::Build(opts)) => {
                println!("{:?}", opts);
            },
            Ok(cli::Command::Clean(opts)) => {
                println!("{:?}", opts);
            },
            Ok(cli::Command::Graph(opts)) => {
                println!("{:?}", opts);
            },
            Err(err) => {
                println!("{}", err);
            },
        };
    }

    println!("Hello, world!");
}
