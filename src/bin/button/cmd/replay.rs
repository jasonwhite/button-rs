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

use std::path::PathBuf;

use button::{logger, Error};
use opts::ColorChoice;

#[derive(StructOpt, Debug)]
pub struct Replay {
    /// Path to the replay file.
    #[structopt(parse(from_os_str))]
    path: PathBuf,

    /// Perform playback in real time. If not specified, playback is
    /// instantaneous.
    #[structopt(long = "realtime")]
    realtime: bool,

    /// Print additional information.
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,

    /// When to colorize the output.
    #[structopt(
        long = "color",
        default_value = "auto",
        raw(
            possible_values = "&ColorChoice::variants()",
            case_insensitive = "true"
        )
    )]
    color: ColorChoice,
}

impl Replay {
    pub fn main(&self) -> Result<(), Error> {
        let logger = logger::Console::new(self.verbose, self.color.into());
        logger::log_from_path(&self.path, logger, self.realtime)?;

        Ok(())
    }
}
