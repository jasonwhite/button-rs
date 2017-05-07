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

/// A resource is an abstract representation of some unit of system state. A
/// resource can be a file, directory, environment variable. The only thing we
/// are interested in doing with a resource is:
///
///  1. Taking its checksum so that we can determine if it has changed.
///  2. Deleting it when it is no longer needed.
///
/// We don't need to know the complete value of a resource's contents.

trait Resource {
    /// Gets the checksum of the resource.
    fn checksum(&self) -> u8[32];

    /// Deletes the resource. Care should be taken to not delete *input*
    /// resources. That is, resources that the build system did not produce.
    fn delete(&self);
}
