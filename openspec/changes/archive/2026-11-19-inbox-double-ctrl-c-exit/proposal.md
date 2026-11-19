# Proposal

## Why

The inbox currently exits on `q`, which makes accidental closure easy during review. Requiring two `Ctrl-C` presses and making `q` and `Esc` show a clear notice reduces accidental exits while keeping the exit action available.

## What Changes

- Require two `Ctrl-C` key presses within a short interval to exit `hd inbox`
- Make `q` and `Esc` keep the inbox open and show "To quit, press Ctrl-C twice"
- Show the same notice after the first `Ctrl-C` while waiting for the second press

## Capabilities

### New Capabilities

None

### Modified Capabilities

- `command-inbox`: Change inbox exit behavior and report the quit shortcut

## Impact

- Updates inbox keyboard handling and help text
- No data, CLI, or dependency changes
