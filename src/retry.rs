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

//! Provides the ability to retry an operation until it succeeds.
//!
//! # Examples
//!
//! ```
//! use std::io;
//!
//! fn find_answer(answer: &str) -> io::Result<()> {
//!
//!     print!("Please enter the correct answer: ")?;
//!
//!     let mut buf = String::new();
//!     io::stdin().read_line(&buf)?;
//!
//!     return if buf == answer {
//!         Ok(())
//!     } else {
//!         Err(io::Error::new(io::ErrorKind::Other, "Sorry, that is not the answer."))
//!     }
//! }
//!
//! let result = Retry::new()
//!     .retries(10)
//!     .delay(Duration::from_secs(2))
//!     .call(|| find_answer("42"));
//! ```

use std::thread::sleep;
use std::time::Duration;
use std::cmp::min;

pub struct Retry {
    /// The number of times to *retry* a function. Note that the function is
    /// always called at least once.
    retries: u32,

    /// The initial duration to wait. For each retry, this is multiplied by the
    /// exponential backoff factor.
    delay: Duration,

    /// The maximum possible delay. If `None`, there is no maximum delay.
    max_delay: Option<Duration>,
}

impl Retry {
    pub fn new() -> Retry {
        Retry {
            retries: 0,
            delay: Duration::from_secs(1),
            max_delay: None,
        }
    }

    /// Sets the number of retries.
    pub fn retries(mut self, retries: u32) -> Retry {
        self.retries = retries;
        self
    }

    /// Sets the initial delay.
    pub fn delay(mut self, delay: Duration) -> Retry {
        self.delay = delay;
        self
    }

    /// Sets the maximum possible delay.
    pub fn max_delay(mut self, max_delay: Duration) -> Retry {
        self.max_delay = Some(max_delay);
        self
    }

    /// Calls the function until it returns a `Ok` result. If an `Ok` result is
    /// never produced, returns the `Result` from the last call to the function
    /// that failed.
    pub fn call<F, T, E>(&self, mut f: F) -> Result<T, E>
        where F: FnMut() -> Result<T, E>
    {
        let mut attempt = self.retries + 1;
        let mut delay = self.delay;

        loop {
            let result = f();

            if result.is_ok() {
                return result;
            }

            attempt -= 1;

            if attempt > 0 {
                sleep(delay);

                // Increase the delay.
                delay = match self.max_delay {
                    Some(max_delay) => min(delay * 2, max_delay),
                    None => delay * 2,
                };
            } else {
                // No more remaining attempts.
                return result;
            }
        }
    }
}
