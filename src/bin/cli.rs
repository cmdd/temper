use clap::{App, Arg};
use std::collections::HashMap;

/// An enumeration over the style of output desired.
arg_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Style {
        Json,
        Line,
        Caret,
    }
}

pub fn cli() -> App<'static, 'static> {
    let arg = |name| {
        Arg::with_name(name)
            .help(DOCS[name].short)
            .long_help(DOCS[name].long)
    };

    let flag = |name| arg(name).long(name);

    App::new("temper")
        .author(crate_authors!())
        .version(crate_version!())
        .about("The speedy, simple, stupid prose linter.")
        .max_term_width(100)
        .arg(arg("file").multiple(true).value_name("PATTERN"))
        .arg(
            flag("lint")
                .short("l")
                .required(true)
                .takes_value(true)
                .number_of_values(1)
                .multiple(true)
                .value_name("PATTERN"),
        )
        .arg(
            flag("output")
                .short("o")
                .takes_value(true)
                .possible_values(&["json", "line", "caret"]),
        )
        .arg(flag("split").short("s").takes_value(true))
}

struct Doc {
    short: &'static str,
    long: &'static str,
}

macro_rules! doc {
    ($map:expr, $name:expr, $short:expr) => {
        doc!($map, $name, $short, $short)
    };
    ($map:expr, $name:expr, $short:expr, $long:expr) => {
        $map.insert($name, Doc {
            short: $short,
            long: concat!($long, "\n "),
        });
    };
}

lazy_static! {
    static ref DOCS: HashMap<&'static str, Doc> = {
        let mut us = HashMap::new();

        doc!(us, "file",
            "The file(s) to check for errors.",
            "The file(s) to run the lints on to check for errors. PATTERN is a \
             glob matching all the files which should be checked. Recursive \
             searches can be done by using glob syntax for recursion. If no \
             file is specified, temper will read from stdin.");

        doc!(us, "lint",
            "The lintset(s) to use to check files.",
            "The lintset(s) used to check files. PATTERN is a glob matching all \
             the files which should be used for linting. Including all lintsets \
             in a folder (recursively or not) can be done through glob syntax. \
             \n\nWhen using a glob, make sure to quote it! Without using quotes, \
             the shell will be in charge of expanding the glob, meaning that \
             the single argument will turn into multiple arguments. Because \
             the lint flag will only take one value (multiple lintsets are passed \
             via repeated use of the flag), the extra arguments will become \
             file arguments rather than lintset arguments.");

        doc!(us, "output",
            "The style in which to print the results.",
            "The style in which to print the results. \
             \n\n`json` will output the results of the lint in json format. \
             This option mostly exists for interoperability with outside \
             programs. \
             \n\n`line` will output the results of the lint in line format, \
             with each suggestion taking up one line (given a sufficiently \
             wide terminal window). In this output mode, the line and column \
             number of the match is given, but the text itself is not printed. \
             \n\n`caret` will output the results of the lint more verbosely, \
             printing out everything `lint` prints, but with each match using \
             multiple lines, and the offending line included in the output, \
             with the match highlighted and underlined with color and carets.");

        doc!(us, "split",
            "The number of partitions that should be made in each file when \
             searching. Can drastically improve performance at the cost of \
             correctness for patterns which match across lines.",
            "The number of partitions that should be made in each file when \
             searching. Searching smaller buffers in parallel is much faster \
             and is easier to run in parallel, so increasing this number can \
             result in drastically improved performance. \
             \n\nHowever, there are a number of things to note with this \
             option. First, increasing this number ad infinitum will not \
             necessarily yield improved performance, since each added split \
             will require each regular expression to be recompiled for that \
             file, which has a performance cost. The best value for this setting \
             will be dependent on your files and your computer (specifically \
             the number of logical cores on your CPU), so some trial and error
             may be necessary to obtain the best possible performance. In \
             addition, this option partitions the file by line. If a match \
             happens to lie on two lines that will be separated after the \
             partition, it will no longer match, yielding incorrect results.");

        us
    };
}
