//! `temper` is a fast and simple tool for checking prose and other writing for
//! syntax and usage errors.

extern crate temper;

extern crate bytecount;
#[macro_use]
extern crate clap;
extern crate crossbeam;
extern crate crossbeam_channel;
extern crate crossbeam_deque;
extern crate failure;
extern crate ignore;
#[macro_use]
extern crate lazy_static;
extern crate memchr;
extern crate memmap;
extern crate num_cpus;
extern crate rayon;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate termcolor;

mod cli;
mod opt;
mod out;

use crossbeam::scope;
use crossbeam_channel::unbounded;
use crossbeam_deque::Deque;
use failure::Error;
use ignore::WalkBuilder;
use memmap::Mmap;
use std::cmp;
use std::fs::File;
use std::result::Result;
use std::str::{self, FromStr};
use termcolor::{BufferWriter, ColorChoice};

use cli::*;
use opt::*;
use out::json::*;
use temper::lint::*;
use temper::prose::*;

const EOL: u8 = b'\n';

struct ProseMmap {
    name: String,
    text: Mmap,
    size: u64,
}

// TODO: multi-line compatibility
fn get_line(clens: &[usize], linum: usize) -> (usize, usize) {
    (clens[linum - 1], clens[linum])
}

fn go(opt: Opt) -> Result<u32, Error> {
    // TODO: stdin
    let mut ls = Vec::new();
    let mut fs = Vec::new();
    let q = Deque::new();

    let split = cmp::max(opt.split, 1);
    let style = opt.style;
    let unicode = opt.unicode;

    // TODO: So much heap allocation ugh
    for l in opt.lints {
        for entry in WalkBuilder::new(&l).build() {
            let mut f = File::open(entry?.path())?;
            let mmap = unsafe { Mmap::map(&f)? };
            let mmap = str::from_utf8(&mmap)?;
            ls.push(Lint::from_str(mmap)?);
        }
    }

    for f in opt.files {
        for entry in WalkBuilder::new(&f).build() {
            let entry = entry?;
            let p = entry.path();
            let f = File::open(p)?;
            let mmap = unsafe { Mmap::map(&f)? };

            fs.push(ProseMmap {
                text: mmap,
                name: p.to_owned().to_string_lossy().to_string(),
                size: f.metadata()?.len(),
            });
        }
    }

    for f in &fs {
        // TODO: [#A] Real offsets

        q.push(Work::Prose(ProseOff {
            prose: Prose {
                name: &f.name,
                text: str::from_utf8(&f.text)?,
                size: f.size,
            },
            offset: Offset::new(0, f.size),
        }))
    }

    let cpus = num_cpus::get();

    let (tx, rx) = unbounded();

    scope(|scope| {
        for _ in 0..cpus {
            q.push(Work::Quit);
            let tx = tx.clone();
            let s = q.stealer();
            let lints = &ls;
            scope.spawn(move || {
                let output = match style {
                    Style::Json => JsonOutput { tx },
                    _ => unimplemented!(),
                };

                let worker = Worker {
                    stealer: s,
                    lints,
                    output,
                };

                worker.exec();
            });
        }
    });

    match style {
        Style::Json => {
            let mut orchestrator = JsonOrchestrator {
                matches: Vec::new(),
                rx,
            };

            orchestrator.print_all(cpus as u8)
        }
        _ => unimplemented!(),
    }
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
