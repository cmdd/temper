# temper: the simple, speedy linter for prose

## TODO
Considered features and things of note:
- [ ] https://www.reddit.com/r/rust/comments/32rjdd/reading_from_a_file_or_stdin_based_on_command/
- [ ] Reliability: CI, tests, rustfmt, clippy
- [ ] verbose output
- [ ] json output
- [ ] glob? (this would deal with the recursion problem)
- [ ] Change the philosophy of this tool altogether, make it more focused on prose? (usage of nlp, etc.)?
- [ ] Should lints have a `[meta]` table to dump meta information? (temper would just store it internally as a `HashMap<String, String>` and spit it out on command)

## Prior art
- proselint
- valelint