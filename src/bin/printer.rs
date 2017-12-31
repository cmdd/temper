use failure::Error;
use std::path::Path;
use std::str;
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

pub struct Printer<W> {
    pub wtr: W,
    pub style: Style,
    pub colors: Colors,
    pub eol: u8,
}

// TODO: Colors!
impl<W: WriteColor> Printer<W> {
    pub fn write_match(&mut self, m: &Match, context: &str, moffset: Offset) -> Result<(), Error> {
        match self.style {
            Style::Line => self.write_match_line(m),
            Style::Caret => self.write_match_caret(m, context, moffset),
            _ => unimplemented!(),
        }
    }

    fn write_match_line(&mut self, m: &Match) -> Result<(), Error> {
        let s = format!(
            "{}:{}:{} {}:{} {}",
            m.file, m.line, m.column, m.lint, m.severity, m.msg
        );

        self.write(s.as_bytes())?;
        self.write_eol(1)
    }

    // TODO: What if we're printing multiple lines?
    fn write_match_caret(
        &mut self,
        m: &Match,
        context: &str,
        moffset: Offset,
    ) -> Result<(), Error> {
        let head = format!("{}: {}", m.severity, m.lint);

        let ds = digits(m.line);
        let file = format!(
            "{:>width$} {}:{}:{}",
            "-->",
            m.file,
            m.line,
            m.column,
            width = ds + 3
        );
        let linum = format!("{} | ", m.line,);
        let msg = format!("{:>width$} {}", "=", m.msg, width = ds + 2);

        self.write(head.as_bytes())?;
        self.write_eol(1)?;
        self.write(file.as_bytes())?;
        self.write_eol(1)?;

        self.write(linum.as_bytes())?;
        self.write(&context[..moffset.start].as_bytes())?;
        if moffset.end >= context.len() {
            self.write(&context[moffset.start..].as_bytes())?;
            self.write_eol(1)?;
            self.write(msg.as_bytes())?;
            self.write_eol(2)
        } else {
            self.write(&context[moffset.start..moffset.end].as_bytes())?;
            self.write(&context[moffset.end..].as_bytes())?;
            self.write_eol(1)?;
            self.write(msg.as_bytes())?;
            self.write_eol(2)
        }
    }

    fn write_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        self.write(path.as_ref().to_string_lossy().as_bytes())
    }

    fn write_eol(&mut self, count: usize) -> Result<(), Error> {
        let eol = self.eol;
        for _ in 0..count {
            self.write(&[eol])?;
        }
        Ok(())
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

fn digits(num: usize) -> usize {
    ((num as f64).log(10.0).floor() + 1.0) as usize
}
