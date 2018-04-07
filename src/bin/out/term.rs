use bytecount;
use failure::Error;
use memchr::memchr;
use std::path::Path;
use std::str;
use termcolor::{ColorSpec, WriteColor};

use temper::lint::*;
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

#[derive(Clone, Debug)]
pub struct CliMatch<'file, 'lint> {
    file: &'file str,
    lint: &'lint str,
    line: u32,
    column: u32,
    severity: Severity,
    msg: &'lint str,
}

pub struct CliPrinter<W> {
    pub wtr: W,
    pub style: Style,
    pub colors: Colors,
    pub eol: u8,
}

// TODO: Colors!
impl<W: WriteColor> CliPrinter<W> {
    pub fn write_match(&mut self, m: &CliMatch, context: &str, moffset: Offset) -> Result<(), Error> {
        match self.style {
            Style::Line => self.write_match_line(m),
            Style::Verbose => self.write_match_verbose(m, context, moffset),
            _ => unimplemented!(),
        }
    }

    fn write_match_line(&mut self, m: &CliMatch) -> Result<(), Error> {
        let s = format!(
            "{}:{}:{} {}:{} {}",
            m.file, m.line, m.column, m.lint, m.severity, m.msg
        );

        self.write(s.as_bytes())?;
        self.write_eol(1)
    }

    fn write_match_verbose(
        &mut self,
        m: &CliMatch,
        context: &str,
        moffset: Offset,
    ) -> Result<(), Error> {
        let head = format!("{}: {}", m.severity, m.lint);

        let nlines = bytecount::count(context.as_bytes(), self.eol) + 1;
        let mut offsets = vec![0];
        let mut last = 0;
        while let Some(i) = memchr(self.eol, context[last..].as_bytes()) {
            offsets.push(i + 1);
            last += i + 1;
        }
        offsets.push(last + context[last..].len());
        offsets.dedup();

        let ds = digits(m.line + nlines - 1);
        let file = format!(
            "{:>width$} {}:{}:{}",
            "-->",
            m.file,
            m.line,
            m.column,
            width = ds + 3
        );

        let msg = format!("{:>width$} {}", "=", m.msg, width = ds + 2);

        self.write(head.as_bytes())?;
        self.write_eol(1)?;
        self.write(file.as_bytes())?;
        self.write_eol(1)?;

        // TODO: Should we pull out regex?
        let context = context.replace('\n', " ").replace('\r', " ");
        let context = context.as_bytes();

        for i in 0..nlines {
            let linum = format!("{:<width$} | ", m.line + i, width = ds);
            self.write(linum.as_bytes())?;
            match moffset.start {
                start if start >= offsets[i] && start < offsets[i + 1] => {
                    match moffset.end {
                        end if end >= offsets[i] && end < offsets[i + 1] => {
                            self.write(&context[offsets[i]..start])?;
                            self.write(&context[start..end])?;
                            self.write(&context[end..offsets[i + 1]])?;
                        }
                        end if end >= offsets[i + 1] => {
                            self.write(&context[offsets[i]..start])?;
                            self.write(&context[start..offsets[i + 1]])?;
                        }
                        _ => {
                            // It's impossible for the end position to be behind
                            // the current line, since by definition the last
                            // line of the context will contain the ending pos
                            unreachable!();
                        }
                    }
                }
                start if start < offsets[i] => {
                    match moffset.end {
                        end if end >= offsets[i] && end < offsets[i + 1] => {
                            self.write(&context[offsets[i]..end])?;
                            self.write(&context[end..offsets[i + 1]])?;
                        }
                        end if end >= offsets[i + 1] => {
                            self.write(&context[offsets[i]..offsets[i + 1]])?;
                        }
                        _ => {
                            // It's impossible for the end position to be behind
                            // the current line, since by definition the last
                            // line of the context will contain the ending pos
                            unreachable!();
                        }
                    }
                }
                _ => {
                    // It's impossible for the start position to be ahead of
                    // the current line, since by definition the first line
                    // includes the start position, and all subsequent lines
                    // will have the start position behind them.
                    unreachable!();
                }
            }
            self.write_eol(1)?;
        }
        self.write(msg.as_bytes())?;
        self.write_eol(2)
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

pub struct CliOrchestrator<'file, 'lint, W> {
    pub matches: Vec<CliMatch<'file, 'lint>>,
    pub printer: CliPrinter<W>,
}