# Proposal

## Why

The inbox only loads events once and closes after a pane jump. Users who keep it open cannot see newly completed commands, and they must reopen it after each jump.

## What Changes

- Make `hd inbox` refresh its event list while it stays open.
- Keep the live inbox open after a pane jump and report jump errors in the inbox.
- Add `hd inbox --oneshot` to keep the current behavior, including closing after a pane jump.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `command-inbox`: define live refresh as the default inbox mode and preserve the current one-shot behavior as an option.

## Impact

- Update inbox CLI parsing, event refresh, jump handling, and related tests.
- Update the README inbox usage and command reference.
- No new dependencies or database changes are expected.
