# Tasks

## 1. Update inbox exit handling

- [x] 1.1 Track Ctrl-C presses in the inbox input loop and exit only after two presses within two seconds; test timing, reset, and exit behavior
- [x] 1.2 Make `q` and `Esc` show "To quit, press Ctrl-C twice" without exiting or changing views; test both keys from list and detail views
- [x] 1.3 Show the quit notice after the first Ctrl-C and update inbox shortcut help; verify the rendered message

## 2. Update documentation and verify

- [x] 2.1 Update README inbox instructions and verify they describe the double Ctrl-C exit behavior
- [x] 2.2 Run `just ci` to verify formatting, Clippy, and the full test suite
