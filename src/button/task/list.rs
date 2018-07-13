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

use std::fmt;
use std::io;
use std::ops;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::any::Any;
use super::traits::{Task, TaskResult};

use res;

/// A list of tasks executed in sequence. This is the root task for all tasks.
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct List {
    list: Vec<Any>,
}

impl List {
    pub fn new(list: Vec<Any>) -> List {
        List { list: list }
    }
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.list.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|v: Vec<Any>| List::new(v))
    }
}

impl From<Vec<Any>> for List {
    fn from(v: Vec<Any>) -> Self {
        List { list: v }
    }
}

impl ops::Deref for List {
    type Target = Vec<Any>;
    fn deref(&self) -> &Vec<Any> {
        &self.list
    }
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.list.len() == 1 {
            write!(f, "{}", self.list[0])
        } else {
            write!(f, "list of {} tasks", self.list.len())
        }
    }
}

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.list.len() == 1 {
            write!(f, "{}", self.list[0])
        } else {
            write!(f, "list of {} tasks", self.list.len())
        }
    }
}

impl Task for List {
    fn execute(&self, root: &Path, log: &mut io::Write) -> TaskResult {
        for task in &self.list {
            task.execute(root, log)?;
        }

        Ok(())
    }

    fn known_inputs(&self, resources: &mut res::Set) {
        for task in &self.list {
            task.known_inputs(resources);
        }
    }

    fn known_outputs(&self, resources: &mut res::Set) {
        for task in &self.list {
            task.known_outputs(resources);
        }
    }
}
