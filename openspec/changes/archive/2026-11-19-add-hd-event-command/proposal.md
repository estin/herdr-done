# Proposal

## Why

Users sometimes need to notify the inbox that work is done without running a command through `hd run`. Today, creating an event requires wrapping a command, which is awkward for work that is already complete or happens outside a shell command.

## What Changes

- Add `hd event MESSAGE` to immediately create a completed inbox event without starting a process
- Support optional `--sound` and `--notify` alerts using the same behavior as `hd run`
- Document the new command and its message in the CLI reference

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `command-inbox`: Add a command that immediately records a user-provided event and can send optional alerts

## Impact

- CLI parsing and dispatch in `src/cli.rs` and `src/main.rs`
- Event creation and persistence, reusing the existing event store and alert delivery
- CLI and behavior tests, plus `README.md`
- No new dependencies or changes to `hd run`
