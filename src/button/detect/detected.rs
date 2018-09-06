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
use std::collections::HashSet;

use res;

/// The sets of detected inputs and outputs of a process.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Detected {
    inputs: HashSet<res::Any>,
    outputs: HashSet<res::Any>,
}

impl Detected {
    pub fn new() -> Detected {
        Detected::default()
    }

    pub fn inputs(&self) -> impl Iterator<Item = &res::Any> {
        self.inputs.iter()
    }

    pub fn outputs(&self) -> impl Iterator<Item = &res::Any> {
        self.outputs.iter()
    }

    pub fn add_input(&mut self, r: res::Any) {
        self.inputs.insert(r);
    }

    #[allow(dead_code)]
    pub fn add_output(&mut self, r: res::Any) {
        self.outputs.insert(r);
    }

    pub fn add(&mut self, other: Detected) {
        self.inputs.extend(other.inputs);
        self.outputs.extend(other.outputs);
    }
}