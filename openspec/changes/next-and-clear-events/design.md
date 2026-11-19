# Design

## Context

The event store already lists completed events newest first and tracks read state. The inbox already selects the next unread event with `n`, but CLI dispatch has no `next` or `clear` command, and the inbox has no delete action. See proposal.md and the `command-inbox` spec delta for the behavior contract.

## Goals / Non-Goals

**Goals:**

- Reuse the event store's existing read state to select and remove events
- Keep unread events protected from both clear entry points
- Share inbox startup behavior between `hd inbox` and `hd next`

**Non-Goals:**

- Add age-based retention or clear filters
- Add confirmation prompts or undo history
- Change read-state behavior or event ordering

## Decisions

### Define `hd next` as the next-unread inbox entry point

Open the existing inbox with unread events, selecting the newest unread item first. This reuses the current inbox navigation and keeps the event available for review. If no unread event exists, report that and exit without starting the TUI. A separate event detail CLI output or automatic pane jump would add behavior not requested here.

### Delete read events only

Use one store operation to delete every event with a read timestamp. Both `hd clear` and the inbox clear action call it, then refresh the displayed events. Keep unread events regardless of age so cleanup cannot discard work that has not been reviewed.

### Use an explicit inbox shortcut

Add `c` as the clear-read action. It provides a simple key that does not conflict with the current `j`, `k`, `n`, `r`, `u`, `J`, and `q` actions. The UI should report how many events it removed and keep its current selection when that event remains.

## Risks / Trade-offs

- [Clear permanently removes events] → Limit deletion to read events and report the number removed
- [Read state can change while the live inbox is open] → Run deletion against current database state and refresh from the store afterward

## Migration Plan

No schema migration is needed because read timestamps already identify removable events. Existing commands and stored events remain valid.
