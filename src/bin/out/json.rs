use crossbeam_channel::{Receiver, Sender};
use failure::Error;
use serde_json;

use temper::prose::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JsonMatch<'file, 'lint> {
    pub fname: &'file str,
    pub lint: &'lint str,
    pub message: &'lint str,
}

pub enum JsonMsg<'file, 'lint> {
    Match(JsonMatch<'file, 'lint>),
    Done,
}

pub struct JsonOutput<'file, 'lint> {
    pub tx: Sender<JsonMsg<'file, 'lint>>,
}

impl<'file, 'lint> Output<'file, 'lint> for JsonOutput<'file, 'lint> {
    type Res = ();
    type Close = ();

    // TODO: [#A] strfmt
    fn exec(&self, m: Match<'file, 'lint>) -> Self::Res {
        let json_match = JsonMatch {
            fname: &m.prose.prose.name,
            lint: &m.lint.name,
            message: &m.lint.msg,
        };

        self.tx.send(JsonMsg::Match(json_match)).unwrap();

        ()
    }

    fn close(self) -> Self::Close {
        self.tx.send(JsonMsg::Done).unwrap();
        drop(self.tx);
        ()
    }
}

pub struct JsonOrchestrator<'file, 'lint> {
    pub matches: Vec<JsonMatch<'file, 'lint>>,
    pub rx: Receiver<JsonMsg<'file, 'lint>>,
}

impl<'file, 'lint> JsonOrchestrator<'file, 'lint> {
    pub fn print_all(self, nrx: u8) -> Result<u32, Error> {
        let res = self.all_to_json(nrx)?;

        println!("{}", res.0);

        Ok(res.1 as u32)
    }

    // TODO: Maybe just wait until everything's closed
    pub fn all_to_json(mut self, nrx: u8) -> Result<(String, u32), Error> {
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

        // TODO: [#A] sort...

        let s = serde_json::to_string(&self.matches).unwrap();

        Ok((s, self.matches.len() as u32))
    }
}
