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
use bincode;
use bit_set;

#[macro_use]
extern crate serde_derive;
use crossbeam;
use failure;
#[macro_use]
extern crate failure_derive;


use holyhashmap;
#[macro_use]
extern crate nom;

use rand;
use reqwest;

use serde_json;
use sha2;
use tempfile;
use termcolor;

mod build;
pub mod build_graph;
mod detect;
pub mod error;
pub mod graph;
pub mod logger;
pub mod res;
pub mod rules;
pub mod state;
pub mod task;
pub mod util;

pub use crate::build::{Build, BuildFailure};
pub use crate::error::{Error, ResultExt};
pub use crate::rules::Rules;
pub use crate::state::BuildState;
