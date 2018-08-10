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

use atty;

use std::str::FromStr;

use termcolor as tc;

/// Global command line options.
#[derive(StructOpt, Debug)]
pub struct GlobalOpts {
    /// When to colorize the output.
    #[structopt(
        long = "color",
        default_value = "auto",
        raw(
            possible_values = "&ColorChoice::variants()",
            case_insensitive = "true"
        )
    )]
    pub color: ColorChoice,
}

/// A color choice.
#[derive(Debug, Copy, Clone)]
pub struct ColorChoice(tc::ColorChoice);

impl ColorChoice {
    pub fn variants() -> [&'static str; 4] {
        ["auto", "always", "ansi", "never"]
    }
}

impl FromStr for ColorChoice {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(ColorChoice(tc::ColorChoice::Auto)),
            "always" => Ok(ColorChoice(tc::ColorChoice::Always)),
            "ansi" => Ok(ColorChoice(tc::ColorChoice::AlwaysAnsi)),
            "never" => Ok(ColorChoice(tc::ColorChoice::Never)),
            _ => Err("invalid color choice"),
        }
    }
}

impl Into<tc::ColorChoice> for ColorChoice {
    fn into(self) -> tc::ColorChoice {
        match self.0 {
            tc::ColorChoice::Auto => {
                // Don't use colors if stdout is piped to a file.
                if atty::is(atty::Stream::Stdout) {
                    tc::ColorChoice::Auto
                } else {
                    tc::ColorChoice::Never
                }
            }
            x @ _ => x,
        }
    }
}
