use failure::Error;

use cli::*;
use temper::prose::*;

struct JsonOutput;

impl Output for JsonOutput {
    type Res = Result<(), Error>;
    
    fn exec(&self, m: Match) -> Self::Res {
        unimplemented!();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct JsonMatch<'a> {
    fname: &'a str,
}

struct JsonPrinter<'file, 'lint> {
    matches: Vec<Match<'file, 'lint>>,
}

impl<'file, 'lint> JsonPrinter<'file, 'lint> {
    
}

