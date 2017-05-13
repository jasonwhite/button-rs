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

use std::io;

/// Placeholder checksum type until we start using a crypto library.
pub type Checksum = [u8; 32];

/// The state associated with a resource. This is stored in the database and
/// used to determine if a resource has changed.
#[allow(dead_code)]
pub enum ResourceState {
    /// The state of the resource has never been computed. In this case, the
    /// resource must *never* be deleted. This state means that the build system
    /// has not taken "ownership" of this resource and has no right to delete
    /// it.
    Unknown,

    /// The resource does not exist.
    Missing,

    /// The resource exists and we have the checksum of its contents.
    Checksum(Checksum),
}

/// FIXME: Use a more abstract error type.
pub type Error = io::Error;

/// A resource is an abstract representation of some unit of system state. A
/// resource can be a file, directory, environment variable. The only thing we
/// are interested in doing with a resource is:
///
///  1. Getting its state so that we can determine if it has changed.
///  2. Deleting it when it is no longer needed.
///
/// A resource is merely an *identifier*. It should not store any state about
/// the actual thing it is referencing. The only state that can be stored with a
/// resource is `ResourceState`.
pub trait Resource {
    /// Gets the state of the resource. This is used to determine if it has
    /// changed.
    fn state(&self) -> Result<ResourceState, Error>;

    /// Deletes the resource. Care should be taken by the caller to not delete
    /// *input* resources. That is, resources that the build system did not
    /// produce. Deleting output resources is perfectly fine.
    fn delete(&self) -> Result<(), Error>;
}
