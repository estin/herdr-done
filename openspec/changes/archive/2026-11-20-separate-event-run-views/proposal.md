# Proposal

## Why

Manually created events and tracked command runs represent different things, but the inbox and detail view present them with the same timing fields. Also, `hd event` prints an ID even when the caller only wants to record a message, which adds unwanted output when used in scripts or shell hooks.

## What Changes

- Show the event's invocation time in inbox list rows for both `hd event` and `hd run`
- Give manually created events a distinct detail view that does not show duration or finish time; preserve the run detail view with its completion timing
- Make `hd event MESSAGE` silent on success by default
- Add an opt-in `--print-id` flag to print the created event ID; storage failures and requested alerts keep their current behavior

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `command-inbox`: event and run list/detail presentation are distinguished, and event creation output is quiet unless `--print-id` is set

## Impact

- `src/event.rs`: add a stored event kind and use it to identify manual events
- `src/store.rs`: add `MIGRATION_V3` and persist the event kind
- `src/inbox.rs`: show a short invocation time in list rows and render event-specific details
- `src/cli.rs`, `src/main.rs`: add and handle `hd event --print-id`
- `README.md`, CLI and inbox tests: document and verify the output and view changes
- No new dependency is expected
