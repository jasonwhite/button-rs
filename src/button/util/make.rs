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
use std::str::FromStr;

use nom::{self, AsChar};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MakeRule {
    pub targets: Vec<String>,
    pub prereqs: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MakeFile(Vec<MakeRule>);

impl FromStr for MakeFile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match make_rules(s) {
            Ok((_, rules)) => Ok(MakeFile(rules)),
            Err(err) => Err(err.to_string()),
        }
    }
}

impl MakeFile {
    pub fn rules(&self) -> &Vec<MakeRule> {
        &self.0
    }
}

fn is_file_name<C: AsChar>(chr: C) -> bool {
    let chr = chr.as_char();
    !(chr == '\\' || chr == ':' || chr == '\r' || chr == '\n' || chr == ' ' ||
      chr == '\0')
}

fn is_space<C: AsChar>(chr: C) -> bool {
    let chr = chr.as_char();
    chr == ' ' || chr == '\t'
}

fn is_rule_separator<C: AsChar>(chr: C) -> bool {
    let chr = chr.as_char();
    chr == '\n' || chr == '\r' || is_space(chr)
}

named!(pub filename<&str, String>,
    escaped_transform!(
        take_while1!(is_file_name),
        '\\',
        alt!(
            tag!("\\") => { |_| "\\" }
            | tag!(" ") => { |_| " " }
            | tag!("r") => { |_| "\r" }
            | tag!("n") => { |_| "\n" }
            | tag!("t") => { |_| "\t" }
            | nom::line_ending => { |_| "" }
        )
    )
);

named!(make_targets<&str, Vec<String>>,
    separated_list!(
        escaped!(
            take_while1!(is_space),
            '\\',
            nom::line_ending
        ),
        filename
    )
);

named!(make_rule<&str, MakeRule>,
    do_parse!(
        targets: make_targets >>
        take_while!(is_space) >>
        tag!(":") >>
        take_while!(is_space) >>
        prereqs: make_targets >>
        (MakeRule { targets, prereqs })
    )
);

named!(make_rules<&str, Vec<MakeRule>>,
    ws!(separated_list!(
        take_while!(is_rule_separator),
        make_rule
    ))
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            filename("\\nfoo\\\\bar\\t\\ baz.o\0"),
            Ok(("\0", String::from("\nfoo\\bar\t baz.o")))
        );

        assert_eq!(
            filename("foo\\\nbar.o\0"),
            Ok(("\0", String::from("foobar.o")))
        );

        assert_eq!(
            filename("foo\\\r\nbar.o\0"),
            Ok(("\0", String::from("foobar.o")))
        );

        assert_eq!(
            make_targets("foo\\ ooo.o bar.o      baz.o:"),
            Ok((
                ":",
                vec!["foo ooo.o".into(), "bar.o".into(), "baz.o".into()]
            ))
        );

        assert_eq!(
            make_rule("foo.o bar.o   :  foo.c \\\n \\\r\n bar.c \n"),
            Ok((
                " \n",
                MakeRule {
                    targets: vec!["foo.o".into(), "bar.o".into()],
                    prereqs: vec!["foo.c".into(), "bar.c".into()],
                }
            ))
        );

        let makefile =
            "foo.o: foo.c foo.h  \t\n  \n\r\n bar.o: bar.c foo.h\n\0";

        assert_eq!(
            make_rules(makefile),
            Ok((
                "\0",
                vec![
                    MakeRule {
                        targets: vec!["foo.o".into()],
                        prereqs: vec!["foo.c".into(), "foo.h".into()],
                    },
                    MakeRule {
                        targets: vec!["bar.o".into()],
                        prereqs: vec!["bar.c".into(), "foo.h".into()],
                    }
                ]
            ))
        );

        assert_eq!(
            make_rules("obj/bar.o: src/bar.c src/foo.h\r\n\0"),
            Ok((
                "\0",
                vec![
                    MakeRule {
                        targets: vec!["obj/bar.o".into()],
                        prereqs: vec!["src/bar.c".into(), "src/foo.h".into()],
                    },
                ]
            ))
        );
    }
}
