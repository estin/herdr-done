# Proposal

## Why

The design includes direct navigation to the next unread completion and cleanup of completed history, but the current CLI does not provide those commands. The inbox also has no way to remove events after they have been reviewed.

## What Changes

- Add `hd next` to open the inbox with the next unread event selected
- Add `hd clear` to remove read events from the event store
- Add an inbox action to clear read events without removing unread items

## Capabilities

### New Capabilities

None

### Modified Capabilities

- `command-inbox`: Add next-unread launch and read-event cleanup through the CLI and inbox

## Impact

- Updates CLI parsing and dispatch, event-store deletion, and inbox interaction
- No new dependencies or database migration are expected
- Clear operations remove read events only; unread events remain available
