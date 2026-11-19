# Design

## Context

`hd run` creates a running event before spawning a child, completes it after the child exits, and sends optional alerts through the shared alert module. `Event::new` accepts a program and arguments, and the store persists completed events. See the proposal and `specs/command-inbox/spec.md` for the user-facing behavior.

## Goals / Non-Goals

**Goals:**

- Reuse the existing event model, persistence, Herdr context detection, and alert delivery
- Keep `hd event` independent of process execution

**Non-Goals:**

- Change `hd run` semantics or alert defaults
- Add event categories, metadata flags, or a background service

## Decisions

- Treat the required message as the event's display label, stored in the existing program field with no arguments. This avoids a database migration and lets list, show, and inbox views display it using their existing command-label formatting.
- Create and complete the event in the command handler, then persist it before sending requested alerts. A storage failure returns failure and suppresses alerts so the user is not told about an event that was not saved.
- Reuse the existing `--sound` and `--notify` flags and alert delivery behavior. Both remain opt-in.
- Accept the message as one positional argument. Users can quote text containing spaces; do not add shell-like message parsing or implicit joining of trailing arguments.

## Risks / Trade-offs

- The event appears as a command label in existing views. This is consistent with the current data model, but may look unusual for prose messages.
- The existing alert title says "Command completed". Reusing it keeps behavior small but is less specific for manually created events.

## Migration Plan

No database migration or deployment step is needed. The event uses the existing schema and remains readable by older versions as a completed event.
