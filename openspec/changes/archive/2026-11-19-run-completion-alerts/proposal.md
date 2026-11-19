# Proposal

## Why

Commands started through `hd run` can finish while the user is focused elsewhere. Opt-in sound and system alerts let users notice completion without changing the existing quiet default.

## What Changes

- Add an opt-in `--sound` flag to `hd run` that plays Herdr's configured completion sound
- Add an opt-in `--notify` flag to `hd run` that sends a system notification when the command finishes
- Keep alert delivery best-effort so it does not change the tracked command's exit result

## Capabilities

### New Capabilities

None

### Modified Capabilities

- `command-inbox`: Allow `hd run` to request completion sound and system notification alerts

## Impact

- Updates the `hd run` CLI and command completion flow
- Adds `notify-rust` for system notifications
- Uses Herdr's configured `done` sound when `--sound` is requested
- Alert delivery remains optional and must not interfere with inherited command streams or exit status
