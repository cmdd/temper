//! `temper` is a fast and simple tool for checking prose and other writing for
//! syntax and usage errors.

extern crate temper;

extern crate bytecount;
#[macro_use]
extern crate clap;
extern crate crossbeam_channel;
extern crate crossbeam_deque;
extern crate failure;
extern crate ignore;
#[macro_use]
extern crate lazy_static;
extern crate memchr;
extern crate memmap;
extern crate rayon;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate termcolor;

mod cli;
mod out;
mod opt;

use failure::Error;
use ignore::WalkBuilder;
use memmap::Mmap;
use rayon::prelude::*;
use std::cmp;
use std::fs::File;
use std::path::Path;
use std::result::Result;
use std::str;
use std::sync::Arc;
use termcolor::{BufferWriter, ColorChoice};

use opt::*;
use out::*;
use temper::lint::*;
use temper::prose::*;

const EOL: u8 = b'\n';

// TODO: multi-line compatibility
fn get_line(clens: &[usize], linum: usize) -> (usize, usize) {
    (clens[linum - 1], clens[linum])
}

fn go(opt: Opt) -> Result<usize, Error> {
    // TODO: stdin
    let mut ls = Vec::new();
    let mut files = Vec::new();

    let split = cmp::max(opt.split, 1);
    let style = opt.style;
    let unicode = opt.unicode;

    // A note on the unquoted glob:
    // When the os expands an unquoted glob, it'll turn into multiple values
    // The -l flag only takes one value per -l, so the rest become arguments
    // as files
    for l in opt.lints {
        for entry in WalkBuilder::new(&l).build() {
            ls.push(entry?.path());
        }
    }

    for f in opt.files {
        for entry in WalkBuilder::new(&f).build() {
            files.push(entry?.path());
        }
    }

    let lints: Lintset = linters(ls.iter().collect())?;

    let bufwtr = Arc::new(BufferWriter::stdout(ColorChoice::Always));

    files
        .par_iter()
        .map(|file| -> Result<usize, Error> {
            let f = File::open(file)?;
            let mmap = unsafe { Mmap::map(&f)? };
            let mmap = str::from_utf8(&mmap)?;

            let bufwtr = Arc::clone(&bufwtr);
            let mut buffer = bufwtr.buffer();
            let prose = Prose {
                name: file.file_name().unwrap().to_str().unwrap(),
                text: mmap,
                split: split,
                unicode: unicode,
                eol: EOL,
            };
            let line_lengths = prose.line_lengths();
            let matches = prose.lint(&lints)?;
            let mut match_count = 0;

            {
                // TODO: Actually use terminal's width
                let mut printer = Printer {
                    wtr: &mut buffer,
                    style: style,
                    colors: Colors::default(),
                    eol: EOL,
                };

                for m in matches {
                    let (ls, le) = get_line(&line_lengths, m.line);
                    let line = &mmap[ls..le].trim_right();
                    let o = Offset {
                        start: m.offset.start - ls,
                        end: m.offset.end - ls,
                    };
                    printer.write_match(&m, line, o)?;
                    match_count += 1;
                }
            }

            bufwtr.print(&buffer)?;

            Ok(match_count)
        })
        .reduce(
            || Ok(0),
            |a, b| match (a, b) {
                (Ok(a), Ok(b)) => Ok(a + b),
                (Err(a), _) => Err(a),
                (_, Err(b)) => Err(b),
            },
        )
}

fn main() {
    let opt = Opt::parse().unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    match go(opt) {
        Ok(c) => {
            println!("{} suggestions found.", c);
        }
        Err(e) => {
            eprintln!("error: {} {}", e.cause(), e.backtrace());
            std::process::exit(1);
        }
    }
}
