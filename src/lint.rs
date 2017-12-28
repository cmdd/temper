// TODO: nomenclature: "ruleset" instead
extern crate toml;

use failure::Error;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use strfmt::strfmt;
use ordermap::OrderMap;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Suggestion,
    Warning,
    Error,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Warning
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize)]
struct TomlLint {
    lint: TomlLintFields,
    #[serde(default = "default_mapping")] mapping: OrderMap<String, Option<String>>,
}

// TODO: A better default msg_mapping
#[derive(Deserialize)]
struct TomlLintFields {
    name: String,
    #[serde(default)] severity: Severity,
    msg: String,
    #[serde(default = "default_msg_mapping")] msg_mapping: String,
    #[serde(default = "default_regex")] regex: String,
    #[serde(default = "default_tokens")] tokens: Vec<String>,
}

#[derive(Debug)]
pub struct Lint {
    pub name: String,
    pub severity: Severity,
    pub msg: String,
    pub msg_mapping: String,
    pub mapping: OrderMap<String, Option<String>>,
}

impl From<TomlLint> for Lint {
    fn from(mut toml: TomlLint) -> Self {
        for token in toml.lint.tokens {
            toml.mapping.insert(token, None);
        }

        let rtemp = toml.lint.regex;
        let mut newmap = OrderMap::new();
        let mut regex = HashMap::with_capacity(1);
        for (item, v) in toml.mapping {
            regex.insert("regex".to_owned(), item);
            newmap.insert(strfmt(&rtemp, &regex).unwrap(), v);
        }

        Lint {
            name: toml.lint.name,
            severity: toml.lint.severity,
            msg: toml.lint.msg,
            msg_mapping: toml.lint.msg_mapping,
            mapping: newmap,
        }
    }
}

pub type Lintset = Vec<Lint>;

// TODO: impl From<Vec<PathBuf>>
pub fn linters<T: AsRef<Path>>(paths: Vec<T>) -> Result<Lintset, Error> {
    let mut res: Lintset = Vec::new();
    for path in paths {
        let mut f = fs::File::open(path)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let lint: Lint = <Lint as From<TomlLint>>::from(toml::from_str(&contents)?);
        res.push(lint);
    }
    Ok(res)
}

fn default_mapping() -> OrderMap<String, Option<String>> {
    OrderMap::new()
}

// TODO
fn default_msg_mapping() -> String {
    String::from("Consider replacing {token} with {value}")
}

fn default_regex() -> String {
    String::from("\\b{regex}\\b")
}

fn default_tokens() -> Vec<String> {
    vec![]
}
