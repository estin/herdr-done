# Tasks

## 1. Quiet event command output

- [x] 1.1 Add `--print-id` to `hd event` and print the created ID only when requested after a successful write; add CLI and integration tests for quiet success, requested ID, and storage failure
- [x] 1.2 Update README event examples and docs tests to show quiet default and `--print-id`

## 2. Inbox event and run presentation

- [x] 2.1 Add a stored event kind and predicate; test it against `hd event` and zero-argument `hd run` records
- [x] 2.2 Show the start/invocation time in every inbox list row; verify both event and run list rows render it
- [x] 2.3 Render event detail without finish time or duration, and preserve all timing fields for run detail; verify with focused render-buffer tests
- [x] 2.4 Document the list and detail differences in README; verify docs tests and CLI help remain accurate

## 3. Integration checks

- [x] 3.1 Run `just ci` and confirm formatting, clippy, and all tests pass
- [x] 3.2 Smoke-test `hd event MESSAGE`, `hd event --print-id MESSAGE`, and both detail types in `hd inbox`

## Workflow follow-up

- Verify with the openspec-verify-change workflow, then archive when requested
