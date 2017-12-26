// TODO: nomenclature: "ruleset" instead
extern crate toml;

use failure::Error;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::fmt;
use std::fs;
use std::io::prelude::*;
use ordermap::OrderMap;

// Should this be user extensible?
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
  Info,
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
  #[serde(default = "default_mapping")]
  mapping: OrderMap<String, Option<String>>,
}

// TODO: A better default msg_mapping
#[derive(Deserialize)]
struct TomlLintFields {
  name: String,
  #[serde(default)]
  severity: Severity,
  msg: String,
  #[serde(default = "default_msg_mapping")]
  msg_mapping: String,
  #[serde(default = "default_tokens")]
  tokens: Vec<String>,
}

#[derive(Debug)]
pub struct Lint {
  pub name: String,
  pub severity: Severity,
  pub msg: String,
  pub msg_mapping: String,
  pub mapping: OrderMap<String, Option<String>>,
  pub len: usize,
}

impl From<TomlLint> for Lint {
  fn from(mut toml: TomlLint) -> Self {
    for token in toml.lint.tokens {
      toml.mapping.insert(token, None);
    }

    let len = &toml.mapping.len();

    Lint {
      name: toml.lint.name,
      severity: toml.lint.severity,
      msg: toml.lint.msg,
      msg_mapping: toml.lint.msg_mapping,
      mapping: toml.mapping,
      len: *len,
    }
  }
}

struct Lintset(Vec<Lint>);

// TODO: impl From<Vec<PathBuf>>
pub fn linters(paths: Vec<PathBuf>, recursive: bool) -> Result<Vec<Lint>, Error> {
  let mut q = VecDeque::from(paths);
  let mut res: Vec<Lint> = Vec::new();
  while !q.is_empty() {
    match q.pop_front().unwrap() {
      ref x if x.is_file() => {
        let mut f = fs::File::open(x)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let lint: Lint = <Lint as From<TomlLint>>::from(toml::from_str(&contents)?);
        res.push(lint);
      },
      ref x if x.is_dir() => {
        for entry in fs::read_dir(x)? {
          let path = entry?.path();
          if path.is_dir() && recursive {
            q.push_back(path);
          } else {
            if path.is_file() {
              q.push_back(path);
            }
          }
        }
      },
      _ => {
        // Here is where we'd return an error
      }
    }
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

fn default_tokens() -> Vec<String> {
  vec![]
}