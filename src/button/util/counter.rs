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

use std::fmt;
use std::ops;

/// A fake writer to count the number of items going into it.
#[allow(dead_code)]
#[derive(Default)]
pub struct Counter {
    count: usize,
}

impl Counter {
    #[allow(dead_code)]
    pub fn new() -> Counter {
        Counter { count: 0 }
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.count
    }
}

impl ops::AddAssign<usize> for Counter {
    fn add_assign(&mut self, rhs: usize) {
        self.count += rhs;
    }
}

impl fmt::Write for Counter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.count += s.len();
        Ok(())
    }
}
