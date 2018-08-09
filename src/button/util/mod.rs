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

mod counter;
mod iter;
mod path;
mod queue;
mod retry;
mod sha256;

pub use self::counter::Counter;
pub use self::iter::empty_or_any;
pub use self::path::PathExt;
pub use self::queue::RandomQueue;
pub use self::retry::{progress_dummy, progress_print, Retry};
pub use self::sha256::{Sha256, ShaVerifyError};

/// A tri-state for checking if we should do things.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Copy, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum NeverAlwaysAuto {
    /// Never do the thing.
    Never,

    /// Always do the thing.
    Always,

    /// Only do the thing under certain circumstances.
    Auto,
}

impl Default for NeverAlwaysAuto {
    /// Never do the thing by default.
    fn default() -> Self {
        NeverAlwaysAuto::Never
    }
}
