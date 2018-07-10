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

use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::slice::{Iter, IterMut};

use serde_json as json;

use res;
use task::{self, Task};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(json::error::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<json::error::Error> for Error {
    fn from(err: json::error::Error) -> Error {
        Error::Parse(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "Failed reading rules: {}", err),
            Error::Parse(ref err) => write!(f, "Failed parsing rules: {}", err),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Parse(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Parse(ref err) => Some(err),
        }
    }
}

/// A rule in the build description. A build description is simply a list of
/// rules.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Rule {
    /// Inputs to the task.
    pub inputs: res::Set,

    /// Outputs from the task.
    pub outputs: res::Set,

    /// The sequence of tasks to execute.
    pub tasks: task::List,
}

#[derive(Debug, PartialEq)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn new(mut rules: Vec<Rule>) -> Rules {
        // Add known inputs and outputs so the user doesn't have to.
        for r in rules.iter_mut() {
            r.tasks.known_inputs(&mut r.inputs);
            r.tasks.known_outputs(&mut r.outputs);
        }

        Rules { rules: rules }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Rules, Error> {
        let f = fs::File::open(path)?;
        Ok(Self::from_reader(io::BufReader::new(f))?)
    }

    pub fn from_reader<R>(reader: R) -> Result<Rules, Error>
    where
        R: io::Read,
    {
        Ok(Self::new(json::from_reader(reader)?))
    }

    #[cfg(test)]
    pub fn from_str<'a>(s: &'a str) -> Result<Rules, Error> {
        Ok(Self::new(json::from_str(s)?))
    }

    pub fn iter(&self) -> Iter<Rule> {
        self.rules.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Rule> {
        self.rules.iter_mut()
    }
}

impl IntoIterator for Rules {
    type Item = Rule;
    type IntoIter = ::std::vec::IntoIter<Rule>;

    fn into_iter(self) -> Self::IntoIter {
        self.rules.into_iter()
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
            "inputs": ["foo.c", "foo.h"],
            "outputs": ["foo.o"],
            "tasks": [
                {
                    "type": "command",
                    "program": "gcc",
                    "args": ["foo.c"]
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
            Command::new(PathBuf::from("gcc"), vec!["foo.c".to_owned()]).into(),
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
