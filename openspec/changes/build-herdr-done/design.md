# Design

## Context

The repository is a new Rust binary with only a placeholder `main.rs`. The requirements are in `specs/command-inbox/spec.md`. See proposal.md for the motivation and scope.

The Herdr CLI and socket API details are not included in this repository. The source design requires checking the installed Herdr version and API schema before selecting exact context and navigation calls.

## Goals / Non-Goals

**Goals:**

- Keep the MVP in one synchronous Rust executable with no daemon.
- Persist event lifecycle and completion details in a local SQLite database.
- Keep child process terminal behavior direct and interactive.
- Make Herdr context optional so `hd` also works in ordinary terminals.

**Non-Goals:**

- Capture command output or environment variables.
- Track commands that users did not start through `hd run`.
- Add notifications, remote synchronization, a Herdr plugin, Nix packaging, or Home Manager integration in this change.

## Decisions

### Run child commands directly

Use the process API with the executable and argument vector. Do not route normal execution through a shell. Inherit stdin, stdout, and stderr. This preserves interactive programs and avoids shell expansion changing the requested command. A separate shell mode is not part of the MVP.

### Use a local SQLite event store

Use SQLite as the only persistent store, with a database under the user's local data directory. Insert a running event before spawning the child, then update it with completion data or a launch failure. This preserves evidence if the wrapper is interrupted. Use short writes, WAL mode, a busy timeout, and migrations. Do not let a database failure change the child's exit result.

Use a sortable unique event ID. Store command arguments as structured data, plus working directory, host, timestamps, duration, process result, read state, and available Herdr identifiers. Do not store environment variables or output. File permissions must restrict the database to its owner.

### Use synchronous Rust

The wrapper waits for one child, the store performs local synchronous operations, and the TUI handles terminal input. Start without an async runtime. Use established CLI, SQLite, terminal UI, and serialization crates when implementing these parts.

### Treat Herdr as an optional public integration

The installed Herdr 0.9.5 exposes `herdr pane current` for current context and a documented `pane.focus` socket API that accepts a pane target. Use these documented operations to capture pane, tab, and workspace identifiers and focus the original pane. Do not depend on Herdr's private database or assume environment variable names. If the API is unavailable, command execution and event storage still work without pane context. Do not attach to the terminal as part of `jump`.

### Keep the MVP focused

Deliver the wrapper and history commands first, then the interactive inbox and documented Herdr navigation. The proposal excludes plugin integration and packaging from this change. Argument redaction, shell execution, retention policy, and output capture are also excluded unless needed to meet a stated requirement.

## Risks / Trade-offs

- [Herdr API differs by installed version] -> Inspect the installed schema and help before selecting calls. Treat missing operations as unavailable integration, not as a reason to fail command execution.
- [The wrapper can be terminated before it updates an event] -> Create a running record before spawning so incomplete records remain detectable.
- [Command arguments can contain secrets] -> Store arguments as required by the event contract, but never store the environment or output. Restrict database file access to the current user. Add redaction only in a later change with a clear policy.
- [Signal handling can change process behavior] -> Test Ctrl+C, termination signals, and child signal exits on supported Unix systems. Preserve child status and document any platform limit.
- [Concurrent writers can contend for SQLite] -> Use WAL mode, a busy timeout, short transactions, and tests with simultaneous completions.

## Migration Plan

There is no existing application data to migrate. Create the data directory and database on first use. Apply schema migrations before each operation. If a migration or database open fails, report the storage error and do not alter the child command's exit result.

