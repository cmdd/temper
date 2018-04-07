use crossbeam_deque::{Deque, Steal, Stealer};
use ordermap::OrderSet;
use regex::{Regex, RegexBuilder, RegexSetBuilder};

use lint::*;

#[derive(Debug, Copy, Clone)]
pub struct Offset {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct File<'text> {
    pub name: String,
    pub text: &'text str,
}

impl<'text> File<'text> {
    // stub
}

#[derive(Debug, Clone, Copy)]
pub struct FileOff<'file> {
    pub file: &'file File<'file>,
    /// The offset of the file to read
    pub offset: Offset,
}

#[derive(Debug, Clone, Copy)]
pub enum Work<'file> {
    File(FileOff<'file>),
    Quit,
}

#[derive(Debug)]
pub struct Worker<'file, 'lints, O> {
    pub stealer: Stealer<Work<'file>>,
    pub lints: &'lints [Lint],
    pub output: O,
}

#[derive(Debug, Clone)]
pub struct Match<'file, 'lint> {
    pub file: FileOff<'file>,
    pub lint: &'lint Lint,
    pub line: u32,
    pub column: u32,
    pub value_str: Option<&'lint str>,
}

// TODO: Should we not duplicate creating the regexes from lints?
//       In this scheme, a worker gets to work on one part of one file with one regex from one lint.
// TODO: Or maybe rayon over the lints and regexes
impl<'file, 'lints, O: Output> Worker<'file, 'lints, O> {
    pub fn exec(&self) {
        loop {
            match self.stealer.steal() {
                Steal::Empty | Steal::Retry => continue,
                Steal::Data(Work::Quit) => break,
                Steal::Data(Work::File(fw)) => {
                    self.search(fw)
                }
            }
        }
    }
    
    fn search(&self, work: FileOff<'file>) {
        for lint in self.lints {
            if let Some(ref token_regex) = lint.tokens {
                let regex = RegexBuilder::new(token_regex).build().unwrap();
                if regex.is_match(work.file.text) {
                    self.search_one(work, lint, regex, None);
                }
            }
            
            let regexes = lint.mapping.iter().map(|x| x.0).collect::<OrderSet<_>>();
            let set = RegexSetBuilder::new(&regexes).build().unwrap();
            
            for regex in set.matches(work.file.text) {
                let regex = regexes.get_index(regex).unwrap();
                let value = lint.mapping.get(*regex).unwrap();
                let regex = RegexBuilder::new(&regex).build().unwrap();
                self.search_one(work, lint, regex, Some(value));
            }
        }
    }
    
    /// Search one file with one regex from one lint.
    /// This is separated out to reduce code duplication and to aid future refactoring efforts (in
    /// case we want to parallelize things further and only have each worker work on this tiny piece
    /// of work)
    fn search_one(&self, work: FileOff<'file>, lint: &Lint, regex: Regex, value: Option<&str>) {
        for mat in regex.find_iter(work.file.text) {
            let (l, c) = unimplemented!();
            let offset = Offset {
                start: work.offset.start + mat.start() as u32,
                end: work.offset.end + mat.end() as u32,
            };

            self.output.exec(Match {
                file: FileOff { offset: offset, ..work },
                lint: &lint,
                line: l,
                column: c,
                value_str: value,
            });
        }
        
    }
}

pub trait Output {
    type Res;

    fn exec(&self, m: Match) -> Self::Res;
}
