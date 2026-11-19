# Proposal

## Why

The database path has no configuration file key and the inbox refresh interval is a hard-coded one second. Users cannot put these in the config file with everything else. And even with a fast interval, a new event shows up to one second late and the inbox burns a query every second while idle. Watching the SQLite WAL file wakes the inbox exactly when a writer commits.

## What Changes

- Add `database` config key: path to the SQLite event store; default stays `$XDG_DATA_HOME/herdr-done/events.db` (or `$HOME/.local/share/...`)
- Add `refresh_interval` config key in human time syntax (e.g. `"1s"`, `"500ms"`, `"2s 500ms"`), parsed with the existing `humantime` dependency; default `"1s"`; zero or unparsable values are configuration errors
- Every live `hd inbox` (including `hd next`) watches the store's WAL file and refreshes immediately when it changes, instead of waiting for the next interval tick
- The configured interval stays as the fallback refresh cadence when no WAL change is seen (e.g. checkpoint truncation, filesystems without usable notifications)
- One-shot mode keeps its current behavior; the watcher only serves the live loop

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `command-inbox`: the layered-configuration requirement gains the `database` and `refresh_interval` keys; the interactive-inbox requirement gains WAL-driven wakeup with the interval as fallback

## Impact

- `Cargo.toml`: add `notify` (filesystem watcher); `humantime` is already a dependency
- `src/config.rs`: two new settings, duration parsing and validation
- `src/store.rs`: expose the resolved database path and its WAL path
- `src/inbox.rs`: watcher-driven refresh in the event loop, interval fallback
- `src/main.rs`: open the store from the resolved config path for all commands
- `README.md`: document both keys and the wakeup behavior
