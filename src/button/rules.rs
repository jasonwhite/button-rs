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

use std::fs;
use std::io;
use std::path::Path;
use std::slice::{Iter, IterMut};

use serde_json as json;

use res;
use task::{self, Task};

#[derive(Debug, Fail)]
pub enum RulesError {
    #[fail(display = "{}", _0)]
    Io(io::Error),

    #[fail(display = "JSON parsing error: {}", _0)]
    Json(json::error::Error),
}

impl From<io::Error> for RulesError {
    fn from(err: io::Error) -> RulesError {
        RulesError::Io(err)
    }
}

impl From<json::error::Error> for RulesError {
    fn from(err: json::error::Error) -> RulesError {
        RulesError::Json(err)
    }
}

/// A rule in the build description. A build description is simply a list of
/// rules.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Rule {
    /// Inputs to the task.
    #[serde(default)]
    pub inputs: res::Set,

    /// Outputs from the task.
    #[serde(default)]
    pub outputs: res::Set,

    /// The sequence of tasks to execute.
    pub tasks: task::List,
}

/// A group of rules.
///
/// Partitions are used to group tasks together to be cached or distributed
/// to another machine.
#[derive(Serialize, Deserialize, Debug)]
pub struct Partition {
    /// The set of inputs for all tasks in the partition. These are used to
    /// calculate the cache key and are also sent to another machine for
    /// distributed builds.
    #[serde(default)]
    inputs: res::Set,

    /// List of rules that are in this partition.
    rules: Vec<Rule>,
}

#[derive(Debug, PartialEq)]
pub struct Rules(Vec<Rule>);

impl Rules {
    pub fn new(mut rules: Vec<Rule>) -> Rules {
        // Add known inputs and outputs so the user doesn't have to.
        for r in &mut rules {
            r.tasks.known_inputs(&mut r.inputs);
            r.tasks.known_outputs(&mut r.outputs);
        }

        Rules(rules)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Rules, RulesError> {
        let f = fs::File::open(path)?;
        Ok(Self::from_reader(io::BufReader::new(f))?)
    }

    pub fn from_reader<R>(reader: R) -> Result<Rules, RulesError>
    where
        R: io::Read,
    {
        Ok(Self::new(json::from_reader(reader)?))
    }

    #[cfg(test)]
    pub fn from_str(s: &str) -> Result<Rules, RulesError> {
        Ok(Self::new(json::from_str(s)?))
    }

    #[cfg(test)]
    pub fn to_string(&self) -> Result<String, json::error::Error> {
        Ok(json::to_string(&self.0)?)
    }

    pub fn iter(&self) -> Iter<Rule> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Rule> {
        self.0.iter_mut()
    }
}

impl IntoIterator for Rules {
    type Item = Rule;
    type IntoIter = ::std::vec::IntoIter<Rule>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use res::FilePath;
    use std::path::PathBuf;
    use task::Command;

    #[test]
    fn test_loading() {
        let data = r#"[{
            "inputs": [{"file": "foo.c"}, {"file": "foo.h"}],
            "outputs": [{"file": "foo.o"}],
            "tasks": [
                {
                    "command": {
                        "program": "gcc",
                        "args": ["foo.c"]
                    }
                }
            ]
        }]"#;

        let rules = Rules::from_str(&data).unwrap();

        let inputs = vec![
            FilePath::from("foo.c").into(),
            FilePath::from("foo.h").into(),
        ];

        let outputs = vec![FilePath::from("foo.o").into()];
        let tasks = vec![
            Command::new(
                PathBuf::from("gcc"),
                vec!["foo.c".into()].into_iter().collect(),
            ).into(),
        ];

        assert_eq!(
            rules,
            Rules::new(vec![Rule {
                inputs: inputs.into_iter().collect(),
                outputs: outputs.into_iter().collect(),
                tasks: tasks.into(),
            }])
        );
    }
}
