use failure::Error;
use memchr::memchr;
use ordermap::OrderSet;
use rayon::prelude::*;
use regex::{Regex, RegexSet};
use std::collections::HashMap;
use std::cmp;
use strfmt::strfmt;

use lint::*;
use util::*;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Offset {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Match {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub lint: String,
    pub severity: Severity,
    pub msg: String,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct Prose<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub split: usize,
    pub eol: u8,
}

impl<'a> Prose<'a> {
    /// Given the index of a character, find its line and column.
    pub fn pos(&self, offset: usize, clens: &[usize], bo: usize) -> (usize, usize) {
        let offset = offset + bo;

        match clens.binary_search(&offset) {
            Ok(linum) => (linum + 1, 1),
            Err(linum) => (linum, offset - clens[linum - 1] + 1),
        }
    }

    // Contains the lengths of each line (and therefore the starting index of each line)
    // item 0 will always be 0, the starting index of line 1
    // item 1 will be the starting index of line 2 (which is also the length of line 1)
    // item 2 will be the starting index of line 3 (which is also the length of lines 1 + 2)
    // ...
    // the last item will be equal to the length of the whole string
    pub fn line_lengths(&self) -> Vec<usize> {
        let nlines = lines(self.text.as_bytes(), self.eol);
        let mut lengths: Vec<usize> = Vec::with_capacity(nlines as usize + 2);

        lengths.push(0);

        let mut current_byte = 0;

        while let Some(pos) = memchr(self.eol, &self.text[current_byte..].as_bytes()) {
            lengths.push(current_byte + pos + 1);
            current_byte = current_byte + pos + 1;
        }

        let last_length = current_byte + self.text[current_byte..].len();
        if self.text[current_byte..].len() > 0 {
            lengths.push(last_length);
        }

        lengths
    }

    pub fn lint(&self, lints: &[Lint]) -> Result<Vec<Match>, Error> {
        let line_lengths = self.line_lengths();
        let nlines = line_lengths.len() - 1;
        let split = cmp::min(nlines, self.split);
        let lps = (nlines as f32 / split as f32).ceil() as usize;

        let mut bytes: Vec<usize> = Vec::with_capacity(self.split + 2);
        bytes.push(0);

        for line in (1..split).map(|i| i * lps) {
            if line % lps == 0 {
                bytes.push(line_lengths[line]);
            }
        }

        // Add 1 to get the whole file to be read; ranges are exclusive at
        // the right end
        bytes.push(*line_lengths.last().unwrap() + 1);

        // This part is pretty unsafe with those square bracket accessors and all
        // Make sure we're good here
        (0..cmp::max((bytes.len() - 1), 1))
            .into_par_iter()
            .map(|s| {
                let buf = if bytes[s] < self.text.len() {
                    if bytes[s + 1] < self.text.len() {
                        &self.text[bytes[s]..bytes[s + 1]]
                    } else {
                        &self.text[bytes[s]..]
                    }
                } else {
                    ""
                };
                let mut nm = self.lint_buf(buf, lints, &line_lengths, bytes[s])?;
                nm.par_sort_unstable_by(|x, y| {
                    if x.line.cmp(&y.line) == cmp::Ordering::Equal {
                        x.column.cmp(&y.column)
                    } else {
                        x.line.cmp(&y.line)
                    }
                });

                Ok(nm)
            })
            .reduce(|| Ok(Vec::new()), bind_extend)
    }

    fn lint_buf(
        &self,
        buf: &str,
        lints: &[Lint],
        clens: &[usize],
        bo: usize,
    ) -> Result<Vec<Match>, Error> {
        let mut regexes: OrderSet<String> = OrderSet::new();
        let mut indivs: HashMap<usize, String> = HashMap::new();
        for (i, lint) in lints.iter().enumerate() {
            for (regex, v) in &lint.mapping {
                match *v {
                    Some(_) => {
                        regexes.insert(regex.clone());
                    }
                    _ => {
                        indivs
                            .entry(i)
                            .or_insert_with(|| format!("(?:{})", regex.clone()))
                            .push_str(&format!("|(?:{})", regex));
                    }
                }
            }
        }

        let res1 = indivs
            .into_par_iter()
            .map(|(i, regex)| -> Result<Vec<Match>, Error> {
                let regex = Regex::new(&regex)?;
                let lint = &lints[i];
                let msg = &lint.msg[..];
                let name = &lint.name[..];
                let mut ires = Vec::new();

                for mat in regex.find_iter(buf) {
                    let (l, c) = self.pos(mat.start(), clens, bo);
                    let bo = Offset {
                        start: bo + mat.start(),
                        end: bo + mat.end(),
                    };
                    let mut map = HashMap::new();
                    map.insert("match".to_string(), &buf[mat.start()..mat.end()]);

                    ires.push(Match {
                        file: String::from(self.name),
                        line: l,
                        column: c,
                        lint: String::from(name),
                        severity: lint.severity,
                        msg: strfmt(msg, &map).unwrap_or_else(|_| String::from(msg)),
                        offset: bo,
                    });
                }

                Ok(ires)
            })
            .reduce(|| Ok(Vec::new()), bind_extend);

        let set = RegexSet::new(&regexes)?;
        let matches: Vec<usize> = set.matches(buf).into_iter().collect();

        let res2 = matches
            .par_iter()
            .map(|rix| -> Result<Vec<Match>, Error> {
                let regex = regexes.get_index(*rix).unwrap();
                let r = Regex::new(regex)?;
                let mut ires = Vec::new();
                for mat in r.find_iter(buf) {
                    for lint in lints {
                        let msg_mapping = &lint.msg_mapping[..];
                        let name = &lint.name[..];
                        if let Some(&Some(ref v)) = lint.mapping.get(regex) {
                            let (l, c) = self.pos(mat.start(), clens, bo);
                            let bo = Offset {
                                start: bo + mat.start(),
                                end: bo + mat.end(),
                            };
                            let mut map = HashMap::new();
                            map.insert("match".to_string(), &buf[mat.start()..mat.end()]);
                            map.insert("value".to_string(), v);

                            ires.push(Match {
                                file: String::from(self.name),
                                line: l,
                                column: c,
                                lint: String::from(name),
                                severity: lint.severity,
                                msg: strfmt(msg_mapping, &map).unwrap_or_else(|_| v.clone()),
                                offset: bo,
                            });
                        }
                    }
                }
                Ok(ires)
            })
            .reduce(|| Ok(Vec::new()), bind_extend);

        bind_extend(res1, res2)
    }
}

fn bind_extend(
    a: Result<Vec<Match>, Error>,
    b: Result<Vec<Match>, Error>,
) -> Result<Vec<Match>, Error> {
    bind(a, b, |a, b| {
        a.iter().chain(b.iter()).cloned().collect::<Vec<_>>()
    })
}
