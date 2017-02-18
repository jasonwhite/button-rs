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
extern crate num_cpus;

pub mod opts;

use std::path::PathBuf;

use clap;

/// Returns clap App arguments to use for parsing.
pub fn app<'a, 'b>() -> clap::App<'a, 'b> {

    // Common options
    let verbose_opt = clap::Arg::with_name("verbose")
        .help("Print additional information.")
        .long("verbose")
        .short("v");

    let file_opt = clap::Arg::with_name("file")
        .help("Path to the build description.")
        .takes_value(true)
        .long("file")
        .short("f");

    let dryrun_opt = clap::Arg::with_name("dryrun")
        .help("Print what might happen.")
        .long("dryrun")
        .short("n");

    let threads_opt = clap::Arg::with_name("threads")
        .help("The number of threads to use.")
        .takes_value(true)
        .long("threads")
        .short("t");

    let color_opt = clap::Arg::with_name("color")
        .help("When to colorize the output.")
        .takes_value(true)
        .long("color")
        .possible_values(&["auto", "never", "always"])
        .default_value("auto");

    let common_opts = &[verbose_opt.clone(), file_opt.clone(),
        dryrun_opt.clone(), threads_opt.clone(), color_opt.clone()];

    clap::App::new("button")
        .version(crate_version!())
        .author(crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(clap::AppSettings::VersionlessSubcommands)
        .subcommand(clap::SubCommand::with_name("build")
            .about("Builds your damn software.")
            .alias("update")
            .args(common_opts)
            .args(&[
                clap::Arg::with_name("auto")
                    .help("Wait for changes and build automatically.")
                    .long("auto"),

                clap::Arg::with_name("watchdir")
                    .help("Used with --auto. The directory to watch for changes in.")
                    .takes_value(true)
                    .long("watchdir"),

                clap::Arg::with_name("delay")
                    .help("Used with --auto. The number of milliseconds to wait before \
                          building.")
                    .takes_value(true)
                    .long("delay")
                    .default_value("50"),
            ])
        )
        .subcommand(clap::SubCommand::with_name("clean")
            .about("Cleans your damn software.")
            .args(common_opts)
            .args(&[
                clap::Arg::with_name("purge")
                    .help("Delete the build state too.")
                    .long("purge"),
            ])
        )
        .subcommand(clap::SubCommand::with_name("graph")
            .about("Graphs your damn software.")
            .arg(file_opt.clone())
            .arg(threads_opt.clone())
            .args(&[
                clap::Arg::with_name("changes")
                    .help("Only display the subgraph that will be traversed on \
                          an update. This has to query the file system for \
                          changes to resources.")
                    .long("changes"),
                clap::Arg::with_name("cached")
                    .help("Display the cached graph from the previous build.")
                    .long("cached"),
                clap::Arg::with_name("full")
                    .help("Displays the full name for each node.")
                    .long("full"),
                clap::Arg::with_name("edges")
                    .help("Type of edges to show.")
                    .takes_value(true)
                    .possible_values(&["explicit", "implicit", "both"])
                    .default_value("explicit")
                    .long("edges"),
            ])
        )
}

pub enum Command {
    Build(opts::Build),
    Clean(opts::Clean),
    Graph(opts::Graph),
}

pub fn subcommand<'a>(name: &str, matches: &clap::ArgMatches<'a>) -> clap::Result<Command> {
    let cpu_count = num_cpus::get();

    match name {
        "build" => Ok(Command::Build(opts::Build {
            file: matches.value_of("file").map(|f| PathBuf::from(f)),
            dryrun: matches.is_present("dryrun"),
            color: try!(value_t!(matches.value_of("color"), opts::Coloring)),
            threads: value_t!(matches, "threads", usize).unwrap_or(cpu_count),
            autobuild: matches.is_present("auto"),
            delay: try!(value_t!(matches, "delay", usize)),
        })),

        "clean" => Ok(Command::Clean(opts::Clean {
            file: matches.value_of("file").map(|f| PathBuf::from(f)),
            dryrun: matches.is_present("dryrun"),
            color: try!(value_t!(matches.value_of("color"), opts::Coloring)),
            threads: value_t!(matches, "threads", usize).unwrap_or(cpu_count),
            purge: matches.is_present("purge"),
        })),

        "graph" => Ok(Command::Graph(opts::Graph {
            file: matches.value_of("file").map(|f| PathBuf::from(f)),
            threads: value_t!(matches, "threads", usize).unwrap_or(cpu_count),
            changes: matches.is_present("changes"),
            cached: matches.is_present("cached"),
            full: matches.is_present("full"),
            edges: try!(value_t!(matches.value_of("edges"), opts::Edges)),
        })),

        // If all subcommands are matched above, then this shouldn't happen.
        _ => unreachable!(),
    }
}
