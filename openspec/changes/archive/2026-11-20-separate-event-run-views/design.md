# Design

## Context

See proposal.md - Why. `Event` has `program`, `args`, `started_at`, `finished_at`, and `duration_ms`. `hd event` stores its message as `program` with no args and completes the event immediately; `hd run` stores the executable and its args. The event kind is not stored yet. Inbox list rows currently show command, result, and duration but not start time. Detail view shows start, finish, and duration for every event. The CLI event command always prints `created event <id>` after storage.

## Goals / Non-Goals

**Goals:**
- Distinguish manually created events from command runs with a small database migration
- Keep successful `hd event` stdout empty unless the caller asks for the ID
- Preserve event timing data in storage for sorting and compatibility

**Non-Goals:**
- Do not remove timestamps or duration from stored event rows
- Do not change `hd run` output or its completion alerts
- Do not add output flags for list or show commands

## Decisions

### D1: Store the event kind
Add `kind` through `MIGRATION_V3` as `TEXT NOT NULL DEFAULT 'run'`. `hd event` writes `event`; `hd run` writes `run`. Old rows default to `run`. `Event::is_manual_event` reads the stored kind. Keep list ordering by `finished_at`; display `started_at` as the invocation time.

Alternative: infer the kind from empty arguments. Rejected because zero-argument command runs such as `date` are common and would lose their run timing.

### D2: Keep the event row unchanged; vary only presentation
For manually created entries, detail view shows message, working directory, result, invocation/start time, and context, but omits finish time and duration. Command runs retain the existing detail fields. Inbox list rows show start time for either kind. Do not change `Event::completed` or null out timing values: storage and ordering continue to use the same fields.

### D3: Explicit ID output flag
Add a boolean `--print-id` on `hd event`, default false. Print the existing `created event <id>` line only when true, after storage and alerts. Do not print it if persistence fails. Alerts remain independent of stdout and keep their existing failure handling. Avoid aliases or alternate output formats.

## Risks / Trade-offs

- [Existing rows have no kind value] → `MIGRATION_V3` gives them `run`, which preserves their run detail view
- [Quiet default may surprise interactive users] → document `--print-id` in README and CLI help
- [Changing event details can hide useful stored fields] → stored start, finish, and duration remain available in `hd show`; only the inbox detail presentation omits finish and duration for manual events
