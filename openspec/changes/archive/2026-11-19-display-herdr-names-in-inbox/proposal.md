# Proposal

## Why

The inbox currently displays Herdr workspace, tab, and pane IDs, which are hard to recognize. Showing their human-readable names will make it easier to find the source location of a command.

## What Changes

- Show the Herdr workspace and tab names in the inbox when available.
- Show a pane name when one exists, and do not show Herdr IDs as location labels.
- Keep Herdr IDs in stored event context for pane navigation and use graceful fallbacks when a name is unavailable.

## Capabilities

### New Capabilities

None

### Modified Capabilities

- `command-inbox`: require readable Herdr location names in inbox list and event details.

## Impact

- Update Herdr context capture and persistence to retain available tab and pane names.
- Update inbox location rendering and tests.
- Existing events without saved names remain readable without displaying IDs as names.
