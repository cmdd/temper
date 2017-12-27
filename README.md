# temper: the simple, speedy linter for prose

## TODO
Considered features and things of note:
- [ ] https://www.reddit.com/r/rust/comments/32rjdd/reading_from_a_file_or_stdin_based_on_command/
- [ ] Reliability: CI, tests, rustfmt, clippy
- [ ] verbose output
- [ ] json output
- [ ] Change the philosophy of this tool altogether, make it more focused on prose? (usage of nlp, etc.)?
- [ ] Should lints have a `[meta]` table to dump meta information? (temper would just store it internally as a `HashMap<String, String>` and spit it out on command)
- [ ] flag for disabling unicode for maximum speed (needed w/ regex template?)
- [ ] Print whole path instead of file name
- [ ] Somehow optimize case where token has no associated value (i.e. compile tokens into a single regexset, because we don't need to know which token matched to print a message)

## Prior art
- proselint
- valelint