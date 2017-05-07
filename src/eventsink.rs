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

use std::time::Duration;

/// Trait for receiving build events, typically for logging or display purposes.
/// For example, printing to the console when a task has been executed.
///
/// Since builds can be recursive, one instantiation of this trait corresponds
/// to just one level in a recursive build.
///
/// These functions cannot fail gracefully. If one of these fails, the only
/// option is to `panic!()`. As such, all initialization that is likely to fail,
/// such as opening a file, should happen while the `EventSink` is being
/// constructed.
trait EventSink {
    /// Called when the build has started.
    fn build_started(&mut self);

    /// Called when the build has succeeded.
    fn build_succeeded(&mut self, duration: Duration);

    /// Called when the build has failed.
    /// TODO: Pass the Error as a parameter.
    fn build_failed(&mut self, duration: Duration);

    // TODO: Add more.
}
