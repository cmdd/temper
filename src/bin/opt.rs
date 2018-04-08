use failure::Error;

use cli::*;

// TODO: field for style
#[derive(Clone, Debug)]
pub struct Opt {
    pub lints: Vec<String>,
    pub style: Style,
    // Note: # of cores / # of files is a good choice
    pub split: usize,
    pub unicode: bool,
    // TODO: If 0 args, read from stdin
    pub files: Vec<String>,
}

impl Opt {
    pub fn parse() -> Result<Opt, Error> {
        let ms = cli().get_matches_safe()?;

        let lints = values_t!(ms, "lint", String)?;
        let style = value_t!(ms, "output", Style).unwrap_or(Style::Line);
        let split = value_t!(ms, "split", usize).unwrap_or(1);
        let files = values_t!(ms, "file", String)?;
        let unicode = !ms.is_present("no-unicode");

        Ok(Opt {
            lints,
            style,
            split,
            files,
            unicode,
        })
    }
}
