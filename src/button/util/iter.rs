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

/// Check if an iterator is empty or if the predicate returns true for any item.
pub fn empty_or_any<I, F>(iter: &mut I, mut f: F) -> bool
where
    I: Iterator,
    F: FnMut(I::Item) -> bool,
{
    match iter.next() {
        None => return true, // Empty
        Some(x) => {
            if f(x) {
                return true;
            }
        }
    };

    for x in iter {
        if f(x) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_or_any() {
        assert!(empty_or_any(&mut [].iter(), |x: &bool| !x));
        assert!(empty_or_any(&mut [true, false, true].iter(), |x| !x));
    }
}
