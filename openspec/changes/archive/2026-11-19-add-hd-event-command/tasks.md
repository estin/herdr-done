# Tasks

## 1. Add the event command

- [x] Add `hd event MESSAGE` with optional `--sound` and `--notify` flags
- [x] Create a completed event with current Herdr context and persist it without spawning a child process
- [x] Report persistence failures and only send alerts after successful persistence

## 2. Verify and document

- [x] Add CLI parsing and integration tests for event creation, required message, alert options, and storage errors
- [x] Document `hd event` and include it in the CLI reference
- [x] Run `just ci`
