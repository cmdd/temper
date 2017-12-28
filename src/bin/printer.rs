use failure::Error;
use termcolor::{ColorSpec, WriteColor};

use temper::prose::*;
use cli::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Colors {
    pub path: ColorSpec,
    pub error: ColorSpec,
    pub warning: ColorSpec,
    pub suggestion: ColorSpec,
    pub matched: ColorSpec,
}

pub struct Printer<W: WriteColor> {
    pub wtr: W,
    pub style: Style,
    pub width: u32,
    pub colors: Colors,
}

impl<W: WriteColor> Printer<W> {
    pub fn write_match(&mut self, m: &Match) -> Result<(), Error> {
        match self.style {
            Style::Line => self.write_match_line(m),
            _ => unimplemented!(),
        }
    }

    // TODO: Colors!
    fn write_match_line(&mut self, m: &Match) -> Result<(), Error> {
        let s = format!(
            "{}:{}:{} {}:{} {}\n",
            m.file, m.line, m.column, m.lint, m.severity, m.msg
        );

        self.write(s.as_bytes())
    }

    fn write_colored<F>(&mut self, buf: &[u8], get_color: F) -> Result<(), Error>
    where
        F: Fn(&Colors) -> &ColorSpec,
    {
        self.wtr.set_color(get_color(&self.colors))?;
        self.write(buf)?;
        self.wtr.reset()?;
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.wtr.write_all(buf)?;
        Ok(())
    }
}
