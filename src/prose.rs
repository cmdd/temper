use failure::Error;
use ordermap::OrderSet;
use rayon::prelude::*;
use regex::{Regex, RegexSet};
use std::collections::HashMap;
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
}

#[derive(Debug)]
pub struct Prose<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub clens: &'a [u32],
}

impl<'a> Prose<'a> {
    /// Given the index of a character, find its line and column.
    pub fn pos(&self, offset: usize, bo: usize) -> (u32, u32) {
        let offset = offset as u32 + bo as u32;

        match self.clens.binary_search(&offset) {
            Ok(linum) => {
                let real = walk(linum, &self.clens);
                (real as u32 + 1, 1)
            }
            Err(linum) => (linum as u32, offset - self.clens[linum - 1] + 1),
        }
    }

    // TODO: Compared to sequential, this runs ~14% faster
    // but we can do better (futures-pool, may, Arc<Mutex<T>>)
    pub fn lint(&self, lints: &Lintset, bo: usize) -> Result<Vec<Match>, Error> {
        let bind = |a: Result<Vec<Match>, Error>,
                    b: Result<Vec<Match>, Error>|
         -> Result<Vec<Match>, Error> {
            match (a, b) {
                (Ok(mut va), Ok(vb)) => {
                    va.extend(vb);
                    Ok(va)
                }
                (Err(x), _) => Err(x),
                (_, Err(y)) => Err(y),
            }
        };

        let mut regexes: OrderSet<String> = OrderSet::new();
        for lint in lints {
            for (regex, _) in &lint.mapping {
                regexes.insert(regex.clone());
            }
        }

        let set = RegexSet::new(&regexes)?;
        let matches: Vec<usize> = set.matches(self.text).into_iter().collect();

        let res = matches
            .par_iter()
            .map(|rix| -> Result<Vec<Match>, Error> {
                let regex = regexes.get_index(*rix).unwrap();
                let r = Regex::new(regex)?;
                let mut ires = Vec::new();
                for mat in r.find_iter(self.text) {
                    for lint in lints {
                        let msg = &lint.msg[..];
                        let msg_mapping = &lint.msg_mapping[..];
                        let name = &lint.name[..];
                        match lint.mapping.get(regex) {
                            Some(&Some(ref v)) => {
                                let (l, c) = self.pos(mat.start(), bo);
                                let mut map = HashMap::new();
                                map.insert("token".to_string(), &self.text[mat.start()..mat.end()]);
                                map.insert("value".to_string(), v);

                                ires.push(Match {
                                    file: String::from(self.name),
                                    line: l,
                                    column: c,
                                    lint: String::from(name),
                                    severity: lint.severity,
                                    msg: strfmt(msg_mapping, &map).unwrap_or(v.clone()),
                                });
                            }
                            Some(&None) => {
                                let (l, c) = self.pos(mat.start(), bo);
                                let mut map = HashMap::new();
                                map.insert("token".to_string(), &self.text[mat.start()..mat.end()]);

                                ires.push(Match {
                                    file: String::from(self.name),
                                    line: l,
                                    column: c,
                                    lint: String::from(name),
                                    severity: lint.severity,
                                    msg: strfmt(msg, &map).unwrap_or(String::from(msg)),
                                });
                            }
                            None => {}
                        }
                    }
                }
                Ok(ires)
            })
            .reduce(|| Ok(Vec::new()), bind);

        res
    }
}
