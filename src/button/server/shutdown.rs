// Copyright (c) 2019 Jason White
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
use futures::{sync::mpsc::Receiver, try_ready, Async, Future, Poll, Stream};
use std::time::{Duration, Instant};
use tokio::timer::Delay;

/// A message that can be sent to the `Idle` future.
pub enum ShutdownMessage {
    /// Resets the timeout.
    ResetIdle,

    /// Forces the future to resolve.
    Shutdown,
}

/// The reason why the `Shutdown` future resolved.
pub enum ShutdownCause {
    Idle(Duration),
    ShutdownRequested,
}

/// A future that resolves when the given channel receiver has been idle for
/// a certain period of time or when an explicit shutdown message is received.
pub struct Shutdown {
    /// The amount of time to wait before resolving the future.
    timeout: Duration,

    /// The underlying timer future.
    timer: Delay,

    /// A channel to reset the duration.
    reset: Receiver<ShutdownMessage>,
}

impl Shutdown {
    /// The countdown begins from the moment this struct is initialized.
    pub fn new(timeout: Duration, reset: Receiver<ShutdownMessage>) -> Self {
        let timer = Delay::new(Instant::now() + timeout);

        Shutdown {
            timeout,
            timer,
            reset,
        }
    }
}

impl Future for Shutdown {
    type Item = ShutdownCause;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.reset.poll() {
                Ok(Async::NotReady) => {
                    // Nothing received yet. Proceed to poll the timer.
                    break;
                }
                Ok(Async::Ready(Some(message))) => {
                    match message {
                        ShutdownMessage::ResetIdle => {
                            // Reset the timeout.
                            self.timer.reset(Instant::now() + self.timeout);
                        }
                        ShutdownMessage::Shutdown => {
                            return Ok(Async::Ready(
                                ShutdownCause::ShutdownRequested,
                            ));
                        }
                    }
                }
                Ok(Async::Ready(None)) => unreachable!(),
                Err(()) => return Err(()),
            }
        }

        try_ready!(self.timer.poll().map_err(|_| ()));
        Ok(Async::Ready(ShutdownCause::Idle(self.timeout)))
    }
}
