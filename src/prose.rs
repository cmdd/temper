use failure::Error;
use ordermap::OrderSet;
use rayon::prelude::*;
use regex::{Regex, RegexSet};
use std::collections::HashMap;
use std::cmp;
use strfmt::strfmt;

use lint::*;
use util::*;

#[derive(Debug, Serialize)]
pub struct Match {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub lint: String,
    pub severity: Severity,
    pub msg: String,
    pub offset: usize,
}

#[derive(Debug)]
pub struct Prose<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub split: usize,
}

impl<'a> Prose<'a> {
    /// Given the index of a character, find its line and column.
    pub fn pos(&self, offset: usize, clens: &[u32], bo: usize) -> (u32, u32) {
        let offset = offset as u32 + bo as u32;

        match clens.binary_search(&offset) {
            Ok(linum) => {
                let real = walk(linum, clens);
                (real as u32 + 1, 1)
            }
            Err(linum) => (linum as u32, offset - clens[linum - 1] + 1),
        }
    }

    pub fn lint(&self, lints: &[Lint]) -> Result<Vec<Match>, Error> {
        let mut clens: Vec<u32> = vec![0];
        let mut last: u32 = 0;

        let nlines = self.text.lines().count();
        let lps = (nlines as f32 / self.split as f32).ceil() as usize;

        let mut bytes: Vec<usize> = vec![0];
        let mut curby = 0;

        for (i, line) in self.text.split('\n').enumerate() {
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

        if self.split > 1 {
            bytes.push(curby - 1);
        }

        // This part is pretty unsafe with those square bracket accessors and all
        // Make sure we're good here
        (0..cmp::max((bytes.len() - 1), 1))
            .into_par_iter()
            .map(|s| {
                let buf = &self.text[bytes[s]..bytes[s + 1]];
                let mut nm = self.lint_buf(buf, &lints, &clens, bytes[s])?;
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
        clens: &[u32],
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
                    let mut map = HashMap::new();
                    map.insert("match".to_string(), &buf[mat.start()..mat.end()]);

                    ires.push(Match {
                        file: String::from(self.name),
                        line: l,
                        column: c,
                        lint: String::from(name),
                        severity: lint.severity,
                        msg: strfmt(msg, &map).unwrap_or_else(|_| String::from(msg)),
                        offset: mat.start(),
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
                                offset: mat.start(),
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
    bind(a, b, |mut a, b| {
        a.extend(b);
        a
    })
}
