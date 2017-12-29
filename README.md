# temper: the simple, speedy linter for prose

## TODO
Considered features and things of note:
- [ ] proper lintset
- [ ] https://www.reddit.com/r/rust/comments/32rjdd/reading_from_a_file_or_stdin_based_on_command/
- [ ] Reliability: CI, tests, rustfmt, clippy
- [ ] Forgo `RegexSet` entirely and just compile one giant regex per lintset
- [ ] json output
- [ ] Change the philosophy of this tool altogether, make it more focused on prose? (usage of nlp, etc.)?
- [ ] flag for disabling unicode for maximum speed (needed w/ regex template?)
- [ ] Print whole path instead of file name
- [ ] Nicer error handling (print glob errors, deal with regex errors for token field (which gets combined))

## Prior art
- proselint
- valelint