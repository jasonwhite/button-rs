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

use std::path::{Path, PathBuf};

use button::{logger, Error, ResultExt};
use opts::GlobalOpts;

#[derive(StructOpt, Debug)]
pub struct Replay {
    /// Path to the replay file.
    #[structopt(parse(from_os_str))]
    path: Option<PathBuf>,

    /// Perform playback in real time. If not specified, playback is
    /// instantaneous.
    #[structopt(long = "realtime")]
    realtime: bool,

    /// Print additional information.
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,
}

impl Replay {
    pub fn main(self, global: &GlobalOpts) -> Result<(), Error> {
        let path = match &self.path {
            Some(path) => path,
            None => Path::new(".button/log"),
        };

        let logger = logger::Console::new(self.verbose, global.color.into());
        logger::log_from_path(&path, logger, self.realtime).with_context(
            |_| format!("Failed loading binary log from {:?}", path),
        )?;

        Ok(())
    }
}
