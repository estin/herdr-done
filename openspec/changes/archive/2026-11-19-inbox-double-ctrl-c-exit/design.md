# Design

## Context

The inbox handles keyboard input in its event loop and renders a footer for shortcuts or status messages. It currently maps `q` to exit and uses `Esc` to return from details to the list. See proposal.md and the `command-inbox` spec delta for the requested behavior.

## Goals / Non-Goals

**Goals:**

- Make leaving the inbox deliberate
- Keep quit guidance visible when users press familiar exit keys
- Detect two Ctrl-C presses within a bounded interval

**Non-Goals:**

- Change behavior for pane jumps or the one-shot inbox mode
- Add configurable key bindings or timing

## Decisions

### Track the first Ctrl-C in the inbox event loop

Record the time of the first Ctrl-C. A second press within two seconds exits; an expired first press is replaced by a new first press. This avoids using a process-level signal handler for terminal key input and makes the timing testable. Other key presses do not exit the inbox.

### Keep q and Esc in the inbox

Handle `q` and `Esc` as requests for guidance, not as navigation or exit. Show the exact notice "To quit, press Ctrl-C twice" and keep the current view unchanged. This gives the same result in list and detail views.

### Render the quit notice through the existing message line

Reuse the inbox status message instead of adding a new panel or changing the layout. Keep the footer's normal shortcut hints updated to describe double Ctrl-C as the exit action.

## Risks / Trade-offs

- [Terminal or platform key handling may represent Ctrl-C as an interrupt instead of a key event] → Verify behavior with Crossterm on supported platforms and keep exit handling within the TUI input path
- [A short interval may be difficult for some users] → Use a two-second interval and show the exact notice after the first press

## Migration Plan

No stored data changes are needed. Update the README shortcut table with the new exit behavior. Existing users can exit with Ctrl-C twice; `q` and `Esc` no longer exit.
