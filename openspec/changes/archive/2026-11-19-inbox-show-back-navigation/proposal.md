# Proposal

## Why

When a user inspects an event in the inbox, `q` and `Esc` currently show a quit notice instead of returning to the event list. These keys should act as back navigation in the detail view while keeping the existing quit protection in the list view.

## What Changes

- Make `q` and `Esc` return from event details to the event list
- Keep `q` and `Esc` from the list view in the inbox and show the quit notice
- Keep double `Ctrl-C` as the inbox exit action

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `command-inbox`: Define `q` and `Esc` behavior by the active inbox view

## Impact

- Inbox key handling and unit tests in `src/inbox.rs`
- Inbox key documentation in `README.md`
