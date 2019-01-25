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
mod args;
mod cmd;
mod opts;
mod paths;

use std::env;
use std::process::exit;

use crate::args::Args;
use crate::cmd::Server;
use structopt::StructOpt;

fn main() {
    // If the BUTTON_SERVER environment variable is present, skip all command
    // line processing below and spawn the server instead.
    if let Some(var) = env::var_os("BUTTON_SERVER") {
        if var == "1" {
            // Remove the variable so it doesn't get inherited by child
            // processes (incase `button` is run as part of the build).
            env::remove_var("BUTTON_SERVER");

            exit(Args::from_cmd(Server::default().into()).main());
        }
    }

    exit(Args::from_args().main())
}
