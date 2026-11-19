# Design

## Context

See proposal.md - Why. Observed state: `Store::open_default()` resolves the XDG data path inside `store::default_database_path()`; `Store` does not expose its path. Every command in `main.rs` calls `Store::open_default()`; only `open_inbox` loads `config::load()` today. The live loop in `inbox::event_loop` polls crossterm for 200 ms and refreshes when a hard-coded `REFRESH_INTERVAL` (1 s) elapses. The store already runs `journal_mode=WAL`, so every commit touches `<db>-wal`. `humantime` 2.4 is already a dependency (used for duration labels); its `parse_duration` accepts `"2s 500ms"`. Config structs use `#[bonfig::settings]` with `deny_unknown_fields`, so new keys are additive.

## Goals / Non-Goals

**Goals:**
- `database` and `refresh_interval` in the same config file as themes and keys, with today's behavior as defaults
- Prompt refresh on new events in the live inbox, interval as safety net

**Non-Goals:**
- No watching in one-shot mode or non-inbox commands
- No debounce tuning knobs, no recursive directory watching beyond what the WAL needs
- No change to the event schema or store queries

## Decisions

### D1: Settings and precedence
Add to `Settings`: `database: Option<String>` (None = current default path) and `refresh_interval: String` (default `"1s"`). Resolution in `config::resolve`:
- `database`: the config value, then `store::default_database_path()`'s XDG default. Standard `HD__DATABASE` is handled by bonfig's env layer. Keep the XDG logic in `store.rs`, so there is one place that knows the layout. `~/` expansion reuses the existing `expand_path` helper
- `refresh_interval`: `humantime::parse_duration`; error prefixed `refresh_interval:` on parse failure or zero. `Config` gains `database: PathBuf` and `refresh_interval: Duration`

Alternative: a `[store]` table - rejected, two flat keys do not need a nesting level.

### D2: Load config for every command
`main.rs` currently loads config only for the inbox. Move the load to the top of `run()` and pass the resolved database path into `Store::open(path)` at every call site, replacing `Store::open_default()`. This makes `hd run`, `hd event`, `hd list`, `hd show`, `hd next`, `hd clear`, and the jump path all honor the same key. Consequence: an invalid config now fails every command, not just the inbox - acceptable and consistent with "invalid configuration prevents the command from continuing"; a missing config file still means defaults. Alternative: config only for inbox commands - rejected, a `hd run` writing to a different store than `hd inbox` reads would silently split the history.

### D3: WAL wakeup with the notify crate
Add `notify` (recommended-watcher feature set, default features off plus `macos_fsevent` on darwin if needed - implementer takes the smallest feature set that builds on Linux and macOS). In `inbox::run` (live mode only):
- Create a `recommended_watcher` with an `mpsc` channel; watch the database's parent directory non-recursively (the `-wal` file may not exist yet at startup and may be recreated on checkpoint; watching the directory catches create/modify/remove regardless)
- Filter events to paths equal to `<db>-wal` (the store exposes `wal_path()`); ignore other files
- In `event_loop`, after the crossterm poll slice, drain the watcher channel non-blockingly; any WAL event sets a dirty flag; refresh when dirty or when `refresh_interval` elapsed, then clear both
- Watcher creation failure is not fatal: fall back to interval-only refresh (the spec's fallback scenario). No error message spam in the TUI; the interval keeps the inbox correct

Alternative: rusqlite `update_hook` - rejected, it only fires on the writing connection, and the writer is another process. Alternative: polling the WAL mtime - rejected, that is the status quo with extra steps; the user asked for a real watch.

### D4: Keep the loop shape
The loop keeps its 200 ms crossterm poll so keys stay responsive; the watcher channel is drained inside the same iteration, so a WAL change surfaces within one poll slice (~200 ms) instead of one second. The refresh query itself is unchanged (`store.list(100, false)`); a checkpoint or read-mark write also triggers a refresh, which is harmless (idempotent list) and keeps read state fresh across multiple inbox instances.

## Risks / Trade-offs

- [Directory watch fires for the main db and -shm files too] → path filter on `-wal` only; other files ignored
- [notify adds a dependency tree] → it is the standard Rust watcher, platform backends are feature-gated; pinned by Cargo.lock and `--locked` CI
- [Filesystem without inotify/FSEvents (some network mounts)] → watcher creation or watch call fails → interval fallback keeps the inbox working
- [Invalid config now blocks all commands] → documented in README; errors name the key, and no config file means defaults, so existing users are unaffected
