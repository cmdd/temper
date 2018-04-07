use crossbeam_channel::{Sender, Receiver};
use failure::Error;
use serde_json;

use cli::*;
use temper::prose::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct JsonMatch<'file, 'lint> {
    fname: &'file str,
    lint: &'lint str,
}

enum JsonMsg<'file, 'lint> {
    Match(JsonMatch<'file, 'lint>),
    Done,
}

struct JsonOutput<'file, 'lint> {
    tx: Sender<JsonMsg<'file, 'lint>>,
}

impl<'file, 'lint> Output<'file, 'lint> for JsonOutput<'file, 'lint> {
    type Res = Result<(), Error>;
    type Close = Result<(), Error>;

    fn exec(&self, m: Match<'file, 'lint>) -> Self::Res {
        let json_match = JsonMatch {
            fname: &m.file.file.name,
            lint: &m.lint.name,
        };

        self.tx.send(JsonMsg::Match(json_match))?;

        Ok(())
    }

    fn close(&self) -> Self::Close {
        self.tx.send(JsonMsg::Done)?;

        Ok(())
    }
}

struct JsonPrinter<'file, 'lint> {
    matches: Vec<JsonMatch<'file, 'lint>>,
    rx: Receiver<JsonMsg<'file, 'lint>>,
    tx: Sender<JsonMsg<'file, 'lint>>,
}

impl<'file, 'lint> JsonPrinter<'file, 'lint> {
    fn all_to_json(&self, nrx: u8) -> Result<String, Error> {
        let mut done = 0;
        for msg in self.rx.iter() {
            match msg {
                JsonMsg::Match(j) => self.matches.push(j),
                JsonMsg::Done => {
                    done += 1;
                    if done >= nrx {
                        break;
                    }
                }
            }
        }

        // TODO sort...

        let s = serde_json::to_string(&self.matches)?;

        Ok(s)
    }
}
