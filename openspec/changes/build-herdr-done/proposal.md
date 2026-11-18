# Proposal

## Why

Commands started in Herdr panes can finish after the user moves to other work. Their result is easy to miss, and the user must find the original pane to inspect it. A persistent inbox will keep completion events available across Herdr restarts and connect each event to its source pane when possible.

## What Changes

- Add a standalone Rust command named `hd` that runs and records commands.
- Add CLI commands to list and inspect recent completion events.
- Add an interactive inbox with read and unread state, event details, and next-unread navigation.
- Store events in SQLite without a background daemon.
- Capture Herdr pane context through documented interfaces and allow pane navigation when the installed API supports it.
- Keep the MVP local. Do not capture command output or environment variables.

## Capabilities

### New Capabilities
- `command-inbox`: Run commands, persist their completion events, and review those events in an inbox.

### Modified Capabilities
None.

## Impact

- Adds a Rust CLI, SQLite storage, and a terminal inbox.
- Uses Herdr's documented CLI or socket API for optional pane context and navigation.
- Requires validation of the installed Herdr API before implementation selects exact context and focus operations.
- Does not add a daemon, network service, output capture, plugin, packaging, or Home Manager integration in the MVP.
