# Tasks

## 1. Add opt-in run options

- [x] 1.1 Add `--sound` and `--notify` to `hd run` and test that both are opt-in and parse independently
- [x] 1.2 Add completion alert delivery using Herdr's `done` sound and `notify-rust`; test alert selection, completion outcome text, and non-fatal delivery failures
- [x] 1.3 Document the new `hd run` options and verify the help output and docs match their behavior

## 2. Verify command lifecycle

- [x] 2.1 Run the focused runner and CLI tests to verify alerts happen only after child completion and preserve exit codes
- [x] 2.2 Run `just ci` to verify formatting, Clippy, and all tests
