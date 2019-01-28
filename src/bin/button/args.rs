// Copyright (c) 2018 Jason White
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
use std::io::{self, Write};

use structopt::StructOpt;
use termcolor::{self as tc, WriteColor};

use button::Error;

use crate::cmd::Command;
use crate::opts::{ColorChoice, GlobalOpts};

#[derive(StructOpt, Debug)]
pub struct Args {
    #[structopt(flatten)]
    global: GlobalOpts,

    #[structopt(subcommand)]
    cmd: Command,
}

/// Displays an error and its list of causes.
pub fn display_error(
    error: Error,
    color: ColorChoice,
) -> Result<(), io::Error> {
    let mut red = tc::ColorSpec::new();
    red.set_fg(Some(tc::Color::Red));
    red.set_bold(true);

    let mut stdout = tc::StandardStream::stdout(color.into());

    let mut causes = error.iter_chain();

    // Primary error.
    if let Some(cause) = causes.next() {
        stdout.set_color(&red)?;
        write!(&mut stdout, "    Error")?;
        stdout.reset()?;
        writeln!(&mut stdout, ": {}", cause)?;
    }

    // Rest of the causes.
    for cause in causes {
        stdout.set_color(&red)?;
        write!(&mut stdout, "Caused by")?;
        stdout.reset()?;
        writeln!(&mut stdout, ": {}", cause)?;
    }

    writeln!(&mut stdout, "{}", error.backtrace())?;

    Ok(())
}

impl Args {
    // Delegate to a subcommand. If any errors occur, print out the error and
    // its chain of causes.
    pub fn main(self) -> i32 {
        if let Err(error) = self.cmd.main(&self.global) {
            let _ = display_error(error, self.global.color);
            1
        } else {
            0
        }
    }
}
