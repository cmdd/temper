# temper
`temper` is a little command-line tool for linting prose, focusing on simplicity and speed.

## TODO
Possible future features:
- [ ] proper lintset
- [ ] json output
- [ ] https://www.reddit.com/r/rust/comments/32rjdd/reading_from_a_file_or_stdin_based_on_command/
- [ ] Reliability: CI, tests (quickcheck/proptest + fuzz), rustfmt, clippy
- [ ] Change the philosophy of this tool altogether, make it more focused on prose? (usage of nlp, etc.)?
- [ ] Print whole path instead of file name
- [ ] Nicer error handling (print glob errors, deal with regex errors for token field (which gets combined))

## Prior art
- proselint
- valelint