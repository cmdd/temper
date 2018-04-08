use crossbeam_deque::{Steal, Stealer};
use failure::Error;
use ordermap::OrderSet;
use regex::{Regex, RegexBuilder, RegexSetBuilder};

use lint::*;

#[derive(Debug, Copy, Clone)]
pub struct Offset {
    pub start: u64,
    pub end: u64,
}

impl Offset {
    pub fn new(start: u64, end: u64) -> Self {
        Offset { start, end }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Prose<'file> {
    pub name: &'file str,
    pub text: &'file str,
    pub size: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ProseOff<'file> {
    pub prose: Prose<'file>,
    /// The offset of the file to read
    pub offset: Offset,
}

#[derive(Debug, Clone, Copy)]
pub enum Work<'file> {
    Prose(ProseOff<'file>),
    Quit,
}

#[derive(Debug)]
pub struct Worker<'file, 'lint, O> {
    pub stealer: Stealer<Work<'file>>,
    pub lints: &'lint [Lint],
    pub output: O,
}

#[derive(Debug, Clone)]
pub struct Match<'file, 'lint> {
    pub prose: ProseOff<'file>,
    pub lint: &'lint Lint,
    pub line: u32,
    pub column: u32,
    pub value_str: Option<&'lint str>,
}

// TODO: Should we not duplicate creating the regexes from lints?
//       In this scheme, a worker gets to work on one part of one file with one regex from one lint.
// TODO: Or maybe rayon over the lints and regexes
// TODO: Maybe instead of giving the Worker an output struct, just give it the sending side of a
//       channel; this would simplify things & reduce duplicated code
// TODO: [#A] Error handling
impl<'file, 'lint, O: Output<'file, 'lint>> Worker<'file, 'lint, O> {
    pub fn exec(self) {
        loop {
            match self.stealer.steal() {
                Steal::Empty | Steal::Retry => continue,
                Steal::Data(Work::Quit) => break,
                Steal::Data(Work::Prose(fw)) => {
                    if let Err(e) = self.search(fw) {
                        println!("{}", e);
                        break;
                    }
                }
            };
        }

        self.output.close();
    }

    fn search(&self, work: ProseOff<'file>) -> Result<(), Error> {
        for lint in self.lints {
            if let Some(ref token_regex) = lint.tokens {
                let regex = RegexBuilder::new(token_regex).build()?;
                if regex.is_match(work.prose.text) {
                    self.search_one(work, lint, regex, None);
                }
            }

            let regexes = lint.mapping.iter().map(|x| x.0).collect::<OrderSet<_>>();
            let set = RegexSetBuilder::new(&regexes).build()?;

            for regex in set.matches(work.prose.text) {
                let regex = regexes.get_index(regex).unwrap();
                let value = lint.mapping.get(*regex).unwrap();
                let regex = RegexBuilder::new(regex).build()?;
                self.search_one(work, lint, regex, Some(value));
            }
        }

        Ok(())
    }

    /// Search one file with one regex from one lint.
    /// This is separated out to reduce code duplication and to aid future refactoring efforts (in
    /// case we want to parallelize things further and only have each worker work on this tiny piece
    /// of work)
    fn search_one(
        &self,
        work: ProseOff<'file>,
        lint: &'lint Lint,
        regex: Regex,
        value: Option<&'lint str>,
    ) {
        for mat in regex.find_iter(work.prose.text) {
            // TODO: [#A] Proper line/column
            let (l, c) = (1, 1);
            let offset = Offset {
                start: work.offset.start + mat.start() as u64,
                end: work.offset.end + mat.end() as u64,
            };

            // TODO: [#A] strfmt?

            self.output.exec(Match {
                prose: ProseOff { offset, ..work },
                lint,
                line: l,
                column: c,
                value_str: value,
            });
        }
    }
}

pub trait Output<'file, 'lint> {
    type Res;
    type Close;

    fn exec(&self, m: Match<'file, 'lint>) -> Self::Res;
    fn close(self) -> Self::Close;
}
