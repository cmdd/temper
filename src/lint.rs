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

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    #[serde(default = "default_msg")] msg: String,
    #[serde(default = "default_msg_mapping")] msg_mapping: String,
    #[serde(default = "default_regex")] regex: String,
    #[serde(default = "default_tokens")] tokens: Vec<String>,
}

#[derive(Debug, Eq, PartialEq)]
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

fn default_msg() -> String {
    String::from("{match} is a usage error")
}

fn default_msg_mapping() -> String {
    String::from("Consider replacing {match} with {value}")
}

fn default_regex() -> String {
    String::from("(?-u:\\b){regex}(?-u:\\b)")
}

fn default_tokens() -> Vec<String> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLETE: &'static str = "\
[lint]
name = 'temper.test.complete'
severity = 'error'
msg = 'This is a complete toml lintset. Match: {match}'
msg_mapping = 'This is a complete toml lintset. {match}: {value}'
regex = 'f {regex} f'

tokens = ['a', '(?-u:b)', 'c']

[mapping]
breakfast = 'yes'
lunch = 'yes'
dinner = 'true'
dessert = 'false'
";

    const DEFAULTS: &'static str = "\
[lint]
name = 'temper.test.defaults'

tokens = ['k']

[mapping]
hello = 'world'
";

    const UNNAMED: &'static str = "
[lint]
msg = 'This lint has no name, and an error should be returned.'
severity = 'error'

tokens = ['whatever']
";

    #[test]
    fn lint_parse_complete() {
        let mut correct_mapping = OrderMap::new();
        correct_mapping.insert(String::from("f a f"), None);
        correct_mapping.insert(String::from("f (?-u:b) f"), None);
        correct_mapping.insert(String::from("f c f"), None);
        correct_mapping.insert(String::from("f breakfast f"), Some(String::from("yes")));
        correct_mapping.insert(String::from("f lunch f"), Some(String::from("yes")));
        correct_mapping.insert(String::from("f dinner f"), Some(String::from("true")));
        correct_mapping.insert(String::from("f dessert f"), Some(String::from("false")));

        let correct = Lint {
            name: String::from("temper.test.complete"),
            severity: Severity::Error,
            msg: String::from("This is a complete toml lintset. Match: {match}"),
            msg_mapping: String::from("This is a complete toml lintset. {match}: {value}"),
            mapping: correct_mapping,
        };

        assert_eq!(correct, <Lint as From<TomlLint>>::from(toml::from_str(COMPLETE).unwrap()));
    }

    #[test]
    fn lint_parse_defaults() {
        let mut correct_mapping = OrderMap::new();
        correct_mapping.insert(String::from(r"\bk\b"), None);
        correct_mapping.insert(String::from(r"\bhello\b"), Some(String::from("world")));

        let correct = Lint {
            name: String::from("temper.test.defaults"),
            severity: Severity::Warning,
            msg: default_msg(),
            msg_mapping: default_msg_mapping(),
            mapping: correct_mapping,
        };

        assert_eq!(correct, <Lint as From<TomlLint>>::from(toml::from_str(DEFAULTS).unwrap()));
    }

    #[test]
    fn lint_parse_unnamed() {
        assert!(toml::from_str::<TomlLint>(UNNAMED).is_err());
    }
}
