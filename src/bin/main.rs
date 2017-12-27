//! `temper` is a fast and simple tool for checking prose and other writing for
//! syntax and usage errors.

extern crate temper;

extern crate failure;
extern crate glob;
extern crate memmap;
extern crate rayon;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate termcolor;

use failure::Error;
use glob::glob;
use memmap::Mmap;
use rayon::prelude::*;
use std::cmp::{self, Ordering};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::result::Result;
use std::str;
use std::sync::Arc;
use structopt::StructOpt;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use temper::lint::*;
use temper::prose::*;

// TODO: Probably replace with clap... structopt is too ambiguous
// doing -i a b will make it think b is a linter, not our input
// See https://kbknapp.github.io/clap-rs/clap/struct.Arg.html#method.multiple
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "temper", about = "A prose linter.")]
struct Opt {
    #[structopt(short = "i", long = "include", help = "A lint or directory of lints to include")]
    lints: Vec<String>,

    #[structopt(short = "V", long = "verbose", help = "More verbose error messages that are larger and feature caret diagnostics")]
    verbose: bool,

    // TODO: needed? we can rely on the outside world for recursive behavior
    #[structopt(short = "r", long = "recur", help = "Recurse through folders for linters")]
    recurse: bool,

    #[structopt(short = "v", long = "version", help = "Print version info")]
    version: bool,

    // Note: # of cores / # of files is a good choice
    #[structopt(short = "s", long = "split", help = "Split the file into parts when checking. Could improve performance, but could also reduce correctness (for patterns that match across newlines).")]
    split: Option<usize>,

    // TODO: Right now this takes ≥ 0 files; we want it to take ≥ 1
    #[structopt(help = "Input file")]
    input: Vec<String>,
}

fn go(opt: Opt) -> Result<(), Error> {
    let bind =
        |a: Result<Vec<Match>, Error>, b: Result<Vec<Match>, Error>| -> Result<Vec<Match>, Error> {
            match (a, b) {
                (Ok(mut va), Ok(vb)) => {
                    va.extend(vb);
                    Ok(va)
                }
                (Err(x), _) => Err(x),
                (_, Err(y)) => Err(y),
            }
        };

    let split = opt.split;

    // TODO: stdin
    let mut ls = Vec::new();
    let mut fs = Vec::new();
    for l in opt.lints.into_iter() {
        for entry in glob(&l)? {
            ls.push(entry?);
        }
    }
    for f in opt.input.into_iter() {
        for entry in glob(&f)? {
            fs.push(entry?);
        }
    }
    let lints: Lintset = linters(ls.iter().map(PathBuf::from).collect())?;
    let files: Vec<PathBuf> = fs.iter().map(PathBuf::from).collect();

    let bufwtr = Arc::new(BufferWriter::stdout(ColorChoice::Always));

    files
        .par_iter()
        .map(|file| -> Result<(), Error> {
            let mut clens: Vec<u32> = vec![0];
            let mut last: u32 = 0;

            let f = File::open(file)?;
            let mmap = unsafe { Mmap::map(&f)? };
            let contents = str::from_utf8(&mmap)?;

            let bufwtr = bufwtr.clone();
            let mut buffer = bufwtr.buffer();

            match split {
                Some(s) => {
                    let mut nlines = contents.lines().count();
                    let s = cmp::max(s, 1);
                    let lps = (nlines as f32 / s as f32).ceil() as usize;

                    let mut bytes: Vec<usize> = vec![0];
                    let mut curby = 0;

                    for (i, line) in contents.split('\n').enumerate() {
                        if i % lps == 0 && i != 0 {
                            bytes.push(curby);
                        }
                        let llen = line.len() as u32;
                        let blen = line.as_bytes().len();
                        // We add one to each of these to account for the newline, which is one byte
                        clens.push(last + llen + 1);
                        last += llen + 1;
                        curby += blen + 1;
                    }
                    bytes.push(curby - 1);

                    // This part is pretty unsafe with those square bracket accessors and all
                    // Make sure we're good here
                    let res = (0..cmp::max((bytes.len() - 1), 1))
                        .into_par_iter()
                        .map(|s| {
                            let file = Prose {
                                name: file.file_name().unwrap().to_str().unwrap(),
                                text: &contents[bytes[s]..bytes[s + 1]],
                                clens: &clens[..],
                            };

                            let mut nm = file.lint(&lints, bytes[s])?;

                            nm.sort_by(|x, y| {
                                if x.line.cmp(&y.line) == Ordering::Equal {
                                    x.column.cmp(&y.column)
                                } else {
                                    x.line.cmp(&y.line)
                                }
                            });

                            Ok(nm)
                        })
                        .reduce(|| Ok(Vec::new()), &bind);

                    match res {
                        Ok(r) => for m in r {
                            writeln!(
                                &mut buffer,
                                "{}:{}:{} {}:{} {}",
                                m.file, m.line, m.column, m.lint, m.severity, m.msg
                            )?;
                        },
                        Err(e) => {
                            return Err(e);
                        }
                    };

                    bufwtr.print(&buffer)?;
                    Ok(())
                }
                None => {
                    let mut clens: Vec<u32> = contents
                        .split('\n')
                        .scan(0, |s, i| {
                            *s = *s + i.len() as u32 + 1;
                            Some(*s)
                        })
                        .collect();
                    clens.insert(0, 0);

                    let file = Prose {
                        name: file.file_name().unwrap().to_str().unwrap(),
                        text: contents,
                        clens: &clens[..],
                    };

                    let mut nm = file.lint(&lints, 0)?;

                    nm.sort_by(|x, y| {
                        if x.line.cmp(&y.line) == Ordering::Equal {
                            x.column.cmp(&y.column)
                        } else {
                            x.line.cmp(&y.line)
                        }
                    });

                    for m in nm {
                        writeln!(
                            &mut buffer,
                            "{}:{}:{} {}:{} {}",
                            m.file, m.line, m.column, m.lint, m.severity, m.msg
                        )?;
                    }

                    bufwtr.print(&buffer)?;

                    Ok(())
                }
            }
        })
        .reduce(
            || Ok(()),
            |a, b| {
                if (a.is_ok() && b.is_ok()) || a.is_err() {
                    a
                } else {
                    b
                }
            },
        )
}

fn main() {
    let opt = Opt::from_args();
    let mopt = opt.clone();

    if mopt.version {
        println!("temper v0.1.0 (2017-12-25)");
        std::process::exit(0);
    }

    match go(opt) {
        Err(e) => {
            println!("{:#?}", e);
        }
        _ => {}
    }
}
