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

use rand::{self, Rng};

use std::sync::{Condvar, Mutex};

/// A queue where elements are popped in random order.
///
/// In the literal sense, this is not a queue. It's really more like a hat where
/// items are pulled out in random order. It just has the same interface as a
/// queue.
#[derive(Default)]
pub struct RandomQueue<T> {
    cvar: Condvar,
    queue: Mutex<Vec<T>>,
}

impl<T> RandomQueue<T>
where
    T: Default,
{
    pub fn new() -> RandomQueue<T> {
        RandomQueue::default()
    }

    /// Adds an element to the queue.
    pub fn push(&self, value: T) {
        self.queue.lock().unwrap().push(value);

        // Notify any threads waiting on the queue.
        self.cvar.notify_one();
    }

    /// Push many values at once while only acquiring the queue lock once.
    ///
    /// Returns the number of items that were pushed.
    pub fn push_many<I>(&self, values: I) -> usize
    where
        I: Iterator<Item = T>,
    {
        let mut queue = self.queue.lock().unwrap();
        let mut count = 0;

        for v in values {
            queue.push(v);
            count += 1;
        }

        // Since we're potentially pushing multiple values, we can wake up
        // multiple threads to pop items off the queue.
        self.cvar.notify_all();

        count
    }

    /// Pops a random element. If the queue is empty, waits for an element to
    /// become available.
    pub fn pop(&self) -> T {
        let mut queue = self.queue.lock().unwrap();

        while queue.is_empty() {
            queue = self.cvar.wait(queue).unwrap();
        }

        let index = rand::thread_rng().gen_range(0, queue.len());
        queue.swap_remove(index)
    }
}
