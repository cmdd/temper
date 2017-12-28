//! `temper` is a fast and simple tool for checking prose and other writing for
//! syntax and usage errors.

extern crate temper;

#[macro_use]
extern crate clap;
extern crate failure;
extern crate glob;
#[macro_use]
extern crate lazy_static;
extern crate memmap;
extern crate rayon;
extern crate termcolor;

mod opt;
mod cli;
mod printer;

use failure::Error;
use glob::glob;
use memmap::Mmap;
use rayon::prelude::*;
use std::cmp;
use std::fs::File;
use std::path::PathBuf;
use std::result::Result;
use std::str;
use std::sync::Arc;
use termcolor::{BufferWriter, ColorChoice};

use opt::*;
use printer::*;
use temper::lint::*;
use temper::prose::*;

fn go(opt: Opt) -> Result<usize, Error> {
    // TODO: stdin
    let mut ls = Vec::new();
    let mut fs = Vec::new();

    let split = cmp::max(opt.split, 1);
    let style = opt.style;

    // A note on the unquoted glob:
    // When the os expands an unquoted glob, it'll turn into multiple values
    // The -l flag only takes one value per -l, so the rest become arguments
    // as files
    for l in opt.lints {
        for entry in glob(&l)? {
            ls.push(entry?);
        }
    }

    for f in opt.files {
        for entry in glob(&f)? {
            fs.push(entry?);
        }
    }

    let lints: Lintset = linters(ls.iter().map(PathBuf::from).collect())?;
    let files: Vec<PathBuf> = fs.iter().map(PathBuf::from).collect();

    let bufwtr = Arc::new(BufferWriter::stdout(ColorChoice::Always));

    files
        .par_iter()
        .map(|file| -> Result<usize, Error> {
            let f = File::open(file)?;
            let mmap = unsafe { Mmap::map(&f)? };
            let contents = str::from_utf8(&mmap)?;

            let bufwtr = bufwtr.clone();
            let mut buffer = bufwtr.buffer();
            let prose = Prose {
                name: file.file_name().unwrap().to_str().unwrap(),
                text: contents,
                split: split,
            };

            let matches = prose.lint(&lints)?;

            let mut match_count = 0;
            {
                // TODO: Actually use terminal's width
                let mut printer = Printer {
                    wtr: &mut buffer,
                    style: style,
                    width: 80,
                    colors: Colors::default(),
                };

                for m in matches {
                    printer.write_match(&m)?;
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
