# **Design plan:&#x20;**`herdr-done`**&#x20;— a command completion inbox for Herdr**

I'd build this as a standalone Rust utility, independent of Ovis and independent of Herdr's internal implementation.

The central idea:

**Every command launched through the wrapper produces a persistent completion event. You can review completed commands later, one by one, and jump directly to their original Herdr panes.**

It should feel like a lightweight job notification system, not another task runner.

# **herdr-done**

Illustrative CLI experience

```
$ hd inbox

Completed commands (3 unread)

❯  cargo build --release       ✓  12.4s
   just deploy                  ✓  42.1s
   nix flake check              ✗  18.7s

↑/↓ navigate   Enter inspect
j jump to pane  r mark read
q quit
```

```
$ hd show 01J...

cargo build --release

Workspace: worker-rust
Tab:       backend
Pane:      2-3
Exit code: 0
Duration:  12.4s
Finished:  21:34:12

[j] Jump to pane
[r] Mark as read
[n] Next unread
```

## **1. Architecture**

I recommend three logical components, all shipped in one Rust binary.

**Shell wrapper**

`hd run -- just deploy`

**Command supervisor**

Spawn → wait → capture exit code → collect Herdr context

**Persistent event store**

SQLite · completion history · read/unread state

**Inbox TUI**

Review · inspect · next unread · jump to pane

### **Important design decision: no daemon in the MVP**

Don't introduce a background service initially.

The wrapper itself writes directly to SQLite when the command finishes. The inbox reads the same database.

This gives you:

- One binary.
- No service to supervise.
- No port to expose.
- No network protocol.
- No background process required.
- Persistent notifications across Herdr restarts.

A daemon can be introduced later if you want desktop notifications, cross-machine synchronization, or live monitoring.

## **2. Command-line interface**

I'd use `hd` as the executable name.

The CLI should have a small number of predictable commands.

| Command             | Purpose                              |
| ------------------- | ------------------------------------ |
| `hd run -- COMMAND` | Execute and track a command          |
| `hd inbox`          | Interactive completion inbox         |
| `hd list`           | List recent completions              |
| `hd show ID`        | Inspect one completion               |
| `hd next`           | Open next unread completion          |
| `hd read ID`        | Mark completion as read              |
| `hd unread ID`      | Mark completion as unread            |
| `hd clear`          | Remove old completed events          |
| `hd stats`          | Show unread/total counts             |
| `hd doctor`         | Check database and Herdr integration |

Examples:

```
hd run -- cargo build --release
hd run -- just deploy
hd run -- nix flake check

hd inbox
hd list --unread
hd show latest
hd next
```

Support command arguments without shell parsing:

```
hd run -- cargo test --workspace --all-features
```

If shell syntax is required, explicitly support:

```
hd run --shell 'cargo build && cargo test'
```

The normal mode should always execute the program directly, not through `/bin/sh -c`.

## **3. Wrapper implementation**

This is the most important component.

The wrapper must behave as transparently as possible.

### **Execution lifecycle**

1. Generate a unique event ID.
2. Record the start timestamp.
3. Detect Herdr context.
4. Spawn the requested command.
5. Wait for completion.
6. Capture the exit status.
7. Record the end timestamp and duration.
8. Persist the completion event.
9. Return the original exit status to the caller.

Pseudo-code:

```
fn run(command: &[OsString]) -> Result<ExitStatus> {
    let started_at = Utc::now();

    let context = HerdrContext::detect();

    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .spawn()?;

    let status = child.wait()?;

    let event = CompletionEvent {
        id: EventId::new(),
        command: command.to_vec(),
        started_at,
        finished_at: Utc::now(),
        exit_code: status.code(),
        context,
    };

    store.insert(&event)?;

    Ok(status)
}
```

This is conceptual code; production implementation needs to handle signals, interrupted waits, and database failures.

### **Preserve terminal behavior**

Do not capture stdout/stderr by default.

Use inherited standard streams:

```
.stdin(Stdio::inherit())
.stdout(Stdio::inherit())
.stderr(Stdio::inherit())
```

This is important because:

- Interactive CLIs continue working.
- TUI applications continue working.
- Colors and terminal dimensions are preserved.
- No output buffering.
- No additional output storage.

The wrapper should not become a pseudo-terminal implementation.

### **Exit codes and signals**

Preserve the child process exit status.

For example:

```
hd run -- cargo test
echo $?
```

Should return exactly the command's exit code.

Handle:

- Normal exit.
- Non-zero exit.
- Signal termination.
- Ctrl+C.
- SIGTERM.
- Spawn failures.

A failed command launch should also create a completion event, with a distinct `spawn_error` status.

For Unix, be particularly careful with signal propagation. The wrapper should forward termination signals to the child and avoid leaving orphan processes.

For MVP, document the signal behavior and test it thoroughly.

## **4. Herdr integration**

This is where the tool becomes more useful than a generic command history utility.

Herdr already exposes a CLI and JSON socket API for inspecting and controlling workspaces, tabs and panes. Its documentation recommends CLI wrappers for simple integrations and direct socket subscriptions for long-lived event consumers.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



I would avoid depending on Herdr's private database or reading its session files.

### **4.1 Capture pane identity**

When `hd run` starts, inspect the Herdr environment.

Proposed context:

```
struct HerdrContext {
    session_id: Option<String>,
    workspace_id: Option<String>,
    workspace_name: Option<String>,
    tab_id: Option<String>,
    pane_id: Option<String>,
}
```

Use this priority:

1. Explicit `--pane` override.
2. Herdr's documented current-pane context.
3. Herdr CLI query for the current pane.
4. No Herdr context if executed outside Herdr.

Important: don't invent environment variable names. During implementation, inspect the installed Herdr version and its API schema:

```
herdr api schema --json
herdr --help
herdr pane --help
herdr tab --help
```

The schema is particularly valuable because it is bundled with the installed Herdr binary.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



### **4.2 Jump to pane**

Add:

```
hd jump <event-id>
```

Expected behavior:

1. Load the event.
2. Check that its Herdr session is available.
3. Resolve the original pane.
4. Focus its workspace/tab/pane using the Herdr CLI or API.
5. Optionally attach to the pane's terminal.

I'd implement two separate actions:

- `jump`: navigate/focus the pane in the current Herdr UI.
- `attach`: attach directly to the terminal.

Herdr documents `herdr terminal attach` and `herdr agent attach` as terminal navigation mechanisms. The exact focus-only operation should be selected from the actual installed API schema rather than assumed.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



### **4.3 Stale pane handling**

Panes can disappear.

Store enough context to show useful information even when a pane no longer exists.

For example:

```
cargo build --release

Workspace: worker-rust
Pane:      2-3

[!] Original pane no longer exists.

Command completed successfully.
```

Never delete the completion just because its original pane disappeared.

## **5. SQLite storage**

Use SQLite as the only persistent storage.

Suggested location:

```
~/.local/share/herdr-done/
├── events.db
└── config.toml
```

I recommend `rusqlite` with bundled SQLite for a self-contained binary.

### **Database schema**

```
CREATE TABLE events (
    id              TEXT PRIMARY KEY,

    command         TEXT NOT NULL,
    args_json       TEXT,

    cwd             TEXT NOT NULL,

    started_at      INTEGER NOT NULL,
    finished_at     INTEGER NOT NULL,
    duration_ms     INTEGER NOT NULL,

    exit_code       INTEGER,
    termination     TEXT NOT NULL,

    read_at         INTEGER,

    session_id      TEXT,
    workspace_id    TEXT,
    workspace_name  TEXT,
    tab_id          TEXT,
    pane_id         TEXT,

    hostname        TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_events_unread
ON events(read_at, finished_at DESC);

CREATE INDEX idx_events_pane
ON events(session_id, pane_id);
```

Use UUIDv7 or ULID for event IDs.

I'd use ULID because it provides sortable, time-ordered identifiers.

### **Event lifecycle**

An event has three possible states:

| State    | Meaning                        |
| -------- | ------------------------------ |
| Unread   | Command finished, not reviewed |
| Read     | User inspected the result      |
| Archived | Removed from the active inbox  |

Don't use `DONE` as a database state. The event is already completed when inserted. Read/unread is a separate dimension.

Also, don't automatically mark an event as read just because the user opened the inbox. Mark it read when explicitly requested or when the user opens its detail view.

### **Database reliability**

- Use WAL mode.
- Use a busy timeout.
- Keep transactions short.
- Enable foreign-key enforcement if additional tables are introduced.
- Use a schema migration mechanism.
- Keep event insertion atomic.

The wrapper should still return the original command's exit code if SQLite is unavailable. Report the persistence failure to stderr and optionally write a recovery record to a local fallback file.

## **6. Interactive inbox TUI**

I'd use `ratatui` and `crossterm`.

The TUI should be keyboard-first, with minimal visual complexity.

### **Main screen**

**Command Inbox**

3 unread

UNREAD

ALL · 12

**❯ cargo build --release**

worker-rust / 2-3

0 · 12.4s

**just deploy**

pless / 1-2

0 · 42.1s

**nix flake check**

home-manager / 1-1

1 · 18.7s

↑↓ Navigate · Enter Inspect · j Jump · r Read · n Next · q Quit

Mockup of the intended interface.

### **Detail screen**

Show:

- Full command and arguments.
- Working directory.
- Exit status.
- Start/end time.
- Duration.
- Workspace, tab and pane.
- Hostname.
- Optional captured output, if enabled in the future.

Keyboard shortcuts:

| Key       | Action          |
| --------- | --------------- |
| `j` / `k` | Next / previous |
| `Enter`   | Inspect         |
| `n`       | Next unread     |
| `r`       | Mark read       |
| `u`       | Mark unread     |
| `J`       | Jump to pane    |
| `a`       | Attach to pane  |
| `d`       | Archive         |
| `/`       | Search          |
| `q`       | Quit            |

### **Important UX detail**

After inspecting an event, `n` should open the next unread event, not simply the next event chronologically.

This makes the inbox useful when you return after several hours and have dozens of completed commands.

## **7. Herdr plugin integration**

I would make Herdr plugin integration optional, not mandatory.

Herdr plugins can be implemented in Rust or any executable language. They have a manifest, declared actions, event hooks and pane entrypoints. Herdr provides the CLI and socket API as the integration surface.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://herdr.dev\&sz=32)

herdr+1



This means the same `hd` binary can serve both as a standalone CLI and as a Herdr plugin.

Proposed layout:

```
herdr-done/
├── Cargo.toml
├── Cargo.lock
├── justfile
├── README.md
├── herdr-plugin.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── runner.rs
│   ├── event.rs
│   ├── store.rs
│   ├── inbox.rs
│   ├── herdr.rs
│   ├── config.rs
│   └── signals.rs
└── tests/
    ├── runner.rs
    ├── store.rs
    ├── herdr.rs
    └── integration.rs
```

### **Plugin actions**

Potential manifest actions:

- `open-inbox`
- `next-completion`
- `show-unread-count`

Potential pane entrypoint:

\- Open the interactive inbox directly inside a Herdr pane.

I would not make the plugin responsible for capturing command completion. That remains the wrapper's responsibility.

This avoids coupling the core functionality to Herdr's plugin lifecycle.

## **8. Configuration**

Keep configuration minimal.

`~/.config/herdr-done/config.toml`:

```
[storage]
database = "~/.local/share/herdr-done/events.db"
retention_days = 30

[runner]
record_outside_herdr = true
record_arguments = true
capture_output = false

[inbox]
show_read = false
mark_read_on_inspect = true
newest_first = true

[herdr]
integration = "auto"
jump_after_selection = false
```

A few details:

- `record_outside_herdr`: useful if you also run commands from ordinary terminal windows.
- `record_arguments`: allows hiding potentially sensitive command arguments.
- `capture_output`: disabled by default to avoid storing secrets.
- `retention_days`: cleanup policy, not automatic deletion of unread items.

I'd also support environment overrides:

```
HD__DATABASE=/tmp/hd.db
HD_DISABLE_RECORDING=1
```

### **Shell and Just integration**

Since you use `just`, I'd provide a reusable recipe:

```
# Execute a command and track its completion
hd *args:
    hd run -- {{args}}
```

Usage:

```
just hd cargo build --release
just hd deploy
just hd nix flake check
```

For commands that need shell syntax:

```
deploy:
    hd run --shell 'cargo build --release && ./deploy.sh'
```

You can also define a convenient alias:

```
alias hrun = hd run --
```

Then:

```
hrun cargo test
hrun just deploy
```

**My recommendation:** keep `hd run --` as the canonical interface. Shell aliases and Just recipes should only be convenience layers.

## **9. Rust technology choices**

I'd avoid unnecessary dependencies and asynchronous complexity.

| Area             | Crate                     | Reason                               |
| ---------------- | ------------------------- | ------------------------------------ |
| CLI              | `clap`                    | Good subcommand and argument support |
| TUI              | `ratatui`                 | Mature terminal UI                   |
| Terminal backend | `crossterm`               | Keyboard and terminal handling       |
| SQLite           | `rusqlite`                | Simple synchronous persistence       |
| Serialization    | `serde`                   | Configuration and JSON               |
| Config           | `toml`                    | Human-readable settings              |
| Time             | `chrono`                  | Timestamps and durations             |
| IDs              | `ulid`                    | Sortable event IDs                   |
| Errors           | `anyhow` + `thiserror`    | Application and domain errors        |
| Logging          | `tracing`                 | Diagnostics                          |
| Paths            | `directories`             | XDG-compatible paths                 |
| Signals          | `signal-hook`             | Unix signal handling                 |
| Testing          | `assert_cmd` + `tempfile` | CLI integration tests                |

I'd use stable Rust and avoid Tokio initially.

The application is primarily synchronous:

- Spawn child.
- Wait for child.
- Write SQLite record.
- Render TUI.

No async runtime is needed.

## **10. Security and reliability**

This tool will record commands from development environments, so there are a few things worth addressing from day one.

**Never store environment variables.**

Commands sometimes contain secrets in arguments:

```
hd run -- curl -H "Authorization: Bearer ..."
```

Consider:

- A `--redact-args` option.
- A configurable list of sensitive argument patterns.
- An option to disable argument recording entirely.
- Database permissions restricted to the current user (`0600`).
- No network communication in the MVP.
- No output capture by default.

**Do not make the wrapper a command shell.**

Use `std::process::Command` with direct arguments. This avoids accidental shell interpretation and preserves normal CLI semantics.

**Avoid losing events.**

SQLite insertion happens after the command exits. If the application is killed immediately afterward, the event could be lost. A future improvement could introduce a lightweight start record before spawning and update it on completion.

I would actually include that in the initial implementation:

```
CREATED → RUNNING → COMPLETED
                   → FAILED_TO_SPAWN
                   → INTERRUPTED
```

This also gives you a way to identify commands whose wrapper was terminated unexpectedly.

## **11. Development milestones**

I'd implement this in six phases. Each phase should produce something usable.

1. **MVP: Command wrapper**

   First priority

   Implement:
   - `hd run -- COMMAND`
   - Child process execution.
   - Exit code preservation.
   - Duration measurement.
   - Herdr context detection.
   - SQLite event persistence.
   - `hd list`, `hd show`.

   Acceptance test:
   ```
   hd run -- sh -c 'sleep 2; exit 42'
   echo $?
   hd list
   ```
   Expected: exit code `42`, with a persisted event showing failure.
2. **Inbox TUI**

   Implement:
   - `hd inbox`.
   - Unread/read states.
   - Navigation.
   - Detail screen.
   - Next unread.
   - Archive and cleanup.

   Acceptance test: run 10 commands, open inbox, review all 10 individually, and verify unread count reaches zero.
3. **Herdr navigation**

   Implement:
   - Current pane detection.
   - Workspace/tab/pane association.
   - `hd jump ID`.
   - `hd attach ID`.
   - Stale pane detection.

   Acceptance test: launch a command in one pane, switch to another pane, finish the command and jump back to its original pane.
4. **Just and Nushell integration**

   Implement:
   - Just recipes.
   - Nushell aliases.
   - Shell execution mode.
   - Optional argument redaction.

   Acceptance test: use `just` and Nushell without changing the underlying command's exit status.
5. **Herdr plugin**

   Implement:
   - Plugin manifest.
   - Open inbox action.
   - Next completion action.
   - Optional dedicated inbox pane.

   Acceptance test: install the plugin and invoke the inbox from Herdr.
6. **Nix packaging and release**

   Implement:
   - Nix derivation.
   - Home Manager module.
   - Single binary release artifacts.
   - Linux x86_64 and aarch64.
   - Automated integration tests.

   Acceptance test: install through Home Manager and use it without manually installing runtime dependencies.

## **12. Testing strategy**

There are some edge cases I'd specifically test.

| Test                                    | Expected behavior                            |
| --------------------------------------- | -------------------------------------------- |
| Command exits 0                         | Success event                                |
| Command exits 1                         | Failure event                                |
| Command exits 42                        | Preserve 42                                  |
| Command not found                       | Spawn failure event                          |
| Ctrl+C                                  | Child terminated, event recorded if possible |
| Wrapper killed                          | Running event can be recovered               |
| Herdr unavailable                       | Command still works                          |
| Pane destroyed                          | Event remains accessible                     |
| SQLite busy                             | Retry or report storage failure              |
| Multiple commands finish simultaneously | No lost events                               |
| Same command launched in multiple panes | Separate events                              |

One especially important test:

```
# Launch in pane A
hd run -- sleep 10

# Launch in pane B
hd run -- sleep 3

# Launch in pane C
hd run -- sleep 5
```

When you open the inbox, it should show the actual completion order:

1. Pane B
2. Pane C
3. Pane A

This is why events should be ordered by `finished_at`, not by when the inbox was opened.

## **13. Future features**

I wouldn't implement these initially, but I'd design the event model so they can be added without major changes.

| Feature                     | Benefit                                             |
| --------------------------- | --------------------------------------------------- |
| Desktop notifications       | Notify immediately when a command finishes          |
| ntfy integration            | Receive notifications on Android                    |
| Command groups              | Group multiple commands belonging to one deployment |
| Tags                        | Filter `build`, `test`, `deploy`                    |
| Output preview              | See last 20 lines without attaching                 |
| Repeat command              | Re-run a failed command                             |
| Duration statistics         | Identify slow builds and deployments                |
| Automatic agent integration | Detect supported agent completions without wrapping |
| Remote inbox                | Aggregate completions from multiple machines        |

One particularly useful future feature would be a notification policy:

```
[notifications]
enabled = true
only_failures = false
only_when_unfocused = true
```

That way you won't receive notifications for every successful build while actively working in its pane.

## **14. Important architectural decisions**

These are the decisions I'd lock before implementation:

| Decision             | Choice                   |
| -------------------- | ------------------------ |
| Language             | Rust                     |
| Distribution         | One executable           |
| Database             | SQLite                   |
| Execution            | Direct child process     |
| Output               | Inherit terminal streams |
| UI                   | Ratatui                  |
| Background daemon    | None initially           |
| Herdr integration    | Public CLI/socket API    |
| Completion detection | Explicit wrapper         |
| Event ordering       | Completion timestamp     |
| Notification state   | Persistent unread/read   |
| Networking           | None initially           |

**One limitation worth emphasizing:** the wrapper tracks only commands explicitly launched through `hd run`. It cannot transparently detect every command typed into an existing interactive shell. That would require shell integration or a separate terminal-level monitoring mechanism.

## **15. Suggested implementation task for a coding agent**

I'd give your Rust coding agent the following specification as the initial implementation brief.

# Build `herdr-done`

Implement a standalone Rust CLI called `hd` that provides a persistent completion inbox for arbitrary CLI commands running inside Herdr terminal panes.

## Core requirements

1. Single Rust binary, no runtime dependencies.
2. Use SQLite for persistent event storage.
3. Execute arbitrary commands through `hd run -- COMMAND [ARGS...]`.
4. Preserve stdin, stdout, stderr, exit codes and terminal interactivity.
5. Record command, arguments, working directory, start time, completion time, duration, exit status and Herdr context.
6. Provide an interactive Ratatui inbox with read/unread state.
7. Allow reviewing completions one by one.
8. Support jumping to the original Herdr pane.
9. Work outside Herdr, with missing Herdr context treated as optional.
10. Never require a daemon in the initial version.

## CLI

Implement:

- `hd run -- COMMAND`
- `hd inbox`
- `hd list`
- `hd show ID`
- `hd next`
- `hd read ID`
- `hd unread ID`
- `hd jump ID`
- `hd attach ID`
- `hd clear`
- `hd stats`
- `hd doctor`

## Architecture

Separate the implementation into:

- CLI parsing
- Command execution and signal handling
- Completion event model
- SQLite persistence and migrations
- Herdr API integration
- Interactive inbox UI
- Configuration

Use synchronous Rust. Avoid Tokio unless a concrete requirement emerges.

## Herdr integration

First inspect the actual installed Herdr CLI and its JSON API schema.

Use documented APIs only. Do not inspect or modify Herdr's internal database.

Identify how to retrieve current session, workspace, tab and pane identity and how to focus an existing pane.

Do not invent environment variable names or API operations.

Gracefully support execution outside Herdr.

## Storage

Use XDG paths:

- Configuration: `~/.config/herdr-done/`
- Database: `~/.local/share/herdr-done/`

Store running events before spawning the child and update them when execution finishes.

Use ULID identifiers, SQLite WAL mode and migrations.

Never store environment variables or command output by default.

## TUI

Use Ratatui and Crossterm.

Support:

- Unread completion count.
- Newest-first event listing.
- Keyboard navigation.
- Event detail view.
- Next unread.
- Mark read/unread.
- Jump to original pane.
- Attach to original pane.
- Search and archive.

## Quality requirements

- Preserve child exit status.
- Handle signals and interrupted execution.
- Handle missing Herdr server.
- Handle stale pane references.
- Handle concurrent SQLite writes.
- Do not lose historical events when a pane disappears.
- Include unit and integration tests.
- Provide a Nix package and Home Manager integration.

## Implementation approach

Build incrementally:

1. Command wrapper and SQLite.
2. CLI history commands.
3. Interactive inbox.
4. Herdr context and navigation.
5. Plugin integration.
6. Nix packaging.

Do not implement speculative features before the MVP works.

Before writing code, inspect the Herdr CLI/API documentation, identify exact supported navigation operations, and write a short architecture document with verified API contracts.

## **My final recommendation**

I'd start with **phases 1–3 only**. That gives you the complete core workflow:

`command → completion event → inbox → inspect → jump to pane`.

Don't build a daemon, push notifications, web interface, or agent integrations yet. The persistent inbox is the actual product; everything else is an extension.

Useful technical references:

- [Herdr Socket API](https://github.com/herdrdev/herdr/blob/master/docs/next/website/src/content/docs/socket-api.mdx?utm_source=chatgpt.com)
- [Herdr CLI reference](https://herdr.dev/tui/docs/cli-reference/?utm_source=chatgpt.com)
- [Herdr Plugin documentation](https://herdr.dev/tui/docs/plugins/?utm_source=chatgpt.com)
- [Ovis repository](https://github.com/devswha/ovis?utm_source=chatgpt.com)
