# Tasks

## 1. Configuration

- [x] 1.1 Add `database: Option<String>` and `refresh_interval: String` (default `"1s"`) to `Settings`; extend `Config` with `database: PathBuf` and `refresh_interval: Duration`
- [x] 1.2 Resolve the database path: config `database` (with `~/` expansion) > XDG default; standard `HD__DATABASE` is handled by bonfig; fold the XDG logic into `store::default_database_path`
- [x] 1.3 Parse `refresh_interval` with `humantime::parse_duration`; reject zero and unparsable values with a `refresh_interval:` error
- [x] 1.4 Tests: defaults unchanged, config path wins over XDG default, `HD__DATABASE` wins over config, `"2s 500ms"` parses, zero and garbage error naming the key

## 2. Store and command wiring

- [x] 2.1 Expose the store's database path and a `wal_path()` helper (`<db>-wal`)
- [x] 2.2 Load config once at the top of `main::run` and pass the resolved path to `Store::open` at every call site, replacing `Store::open_default()`
- [x] 2.3 Test: a command with a configured `database` writes where the inbox reads (integration test with a temp path)

## 3. WAL wakeup

- [x] 3.1 Add the `notify` dependency with the smallest feature set that builds on Linux and macOS; refresh Cargo.lock
- [x] 3.2 In live `inbox::run`, watch the database parent directory non-recursively, filter events to the WAL path, and send them through an mpsc channel; watcher failure falls back to interval-only refresh
- [x] 3.3 In `event_loop`, drain the watcher channel each iteration; refresh when a WAL change arrived or `config.refresh_interval` elapsed; remove the hard-coded `REFRESH_INTERVAL`
- [x] 3.4 Tests: a WAL-path event triggers refresh before the interval elapses (drive the loop's refresh decision as a unit); non-WAL paths do not; interval fallback still refreshes; one-shot mode creates no watcher

## 4. Documentation and verification

- [x] 4.1 Document `database`, `refresh_interval`, precedence with `HD__DATABASE`, and the WAL wakeup behavior in README.md
- [x] 4.2 Run `just ci` and fix any findings
- [x] 4.3 Smoke-test: open `hd inbox`, run `hd event "wake test"` in another shell, confirm the list updates in well under one second; repeat with `refresh_interval = "10s"` to prove the wakeup is not the interval

## Workflow follow-up

- Verify with the openspec-verify-change workflow, then archive when the user asks
