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

use std::path::Path;
use std::error;
use std::io;
use std::fs;
use std::fmt;
use std::slice::Iter;

use serde_json as json;

use resources;
use tasks;

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
            Error::Io(ref err)    => write!(f, "Failed reading rules: {}", err),
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
    pub inputs: Vec<resources::FilePath>,

    /// Outputs from the task.
    pub outputs: Vec<resources::FilePath>,

    /// The sequence of commands to execute.
    pub tasks: Vec<tasks::Command>,
}

#[derive(Debug, PartialEq)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Rules, Error> {
        let f = fs::File::open(path)?;
        Ok(Self::from_reader(io::BufReader::new(f))?)
    }

    pub fn from_reader<R>(reader: R) -> Result<Rules, Error>
        where R: io::Read
    {
        Ok(Rules { rules: json::from_reader(reader)? })
    }

    #[cfg(test)]
    pub fn new(rules: Vec<Rule>) -> Rules {
        Rules { rules: rules }
    }

    #[cfg(test)]
    pub fn from_str<'a>(s: &'a str) -> Result<Rules, Error> {
        Ok(Rules { rules: json::from_str(s)? })
    }

    pub fn iter(&self) -> Iter<Rule> {
        self.rules.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resources::FilePath;

    #[test]
    fn test_loading() {

        let data = r#"[{
            "inputs": ["foo.c", "foo.h"],
            "outputs": ["foo.o"],
            "tasks": [
                ["gcc", "foo.c"]
            ]
        }]"#;

        let rules = Rules::from_str(&data).unwrap();

        assert_eq!(rules, Rules::new(vec![Rule {
            inputs: vec![FilePath::from("foo.c"), FilePath::from("foo.h")],
            outputs: vec![FilePath::from("foo.o")],
            tasks: vec![
                vec!["gcc".to_owned(), "foo.c".to_owned()],
            ],
        }]));
    }
}
