# Design

## Context

See proposal.md for motivation and specs/command-inbox/spec.md for the behavior contract. The inbox currently loads up to 100 events once, polls terminal input every 200 ms, and returns a selected event ID to `main` when the user presses `J`. The store can query the latest events and update read state.

## Goals / Non-Goals

**Goals:**

- Refresh the current event list during the existing input loop.
- Keep event selection stable when refreshed results still include the selected event.
- Route live-mode jump results back through the existing pane-focus path without ending the inbox session.
- Preserve the current close-after-jump flow when the user passes `--oneshot`.

**Non-Goals:**

- Add database notifications or a background watcher.
- Make the refresh interval user-configurable.
- Change standalone `hd jump` behavior.

## Decisions

- Poll the store after the existing 200 ms input poll, with a one-second refresh interval. This avoids a new thread or dependency and keeps input responsive.
- Refresh the app's event vector by event ID. Preserve the selected event when it remains in the latest 100 results; otherwise select the first result or leave the empty list at index zero.
- Pass inbox mode from CLI dispatch. In live mode, return a jump action to the caller, focus the pane, then resume the same terminal session and show any focus error as an inbox message. In one-shot mode, keep returning the selected ID to the current close-and-jump flow.
- Keep one-shot behavior explicit in CLI and README help so existing workflows remain available after the default changes.

## Risks / Trade-offs

- [Risk] Frequent database reads can add work while the inbox is idle → A fixed one-second interval limits polling to one query per second.
- [Risk] A selected old event can leave the latest 100 results → Keep the current list until refresh, then select the newest event if the selected ID is no longer present.
