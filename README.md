# herdr-done

`hd` runs a command, stores a completion event, and lets you review the result later. The tool works inside Herdr and in an ordinary terminal.

## Build

```sh
cargo build --release
```

The executable is `target/release/hd`.

## Run a command

```sh
hd run -- cargo test --workspace
hd run -- just deploy
hd run -- nix flake check
```

`hd run --` passes the executable and arguments directly. The command does not use a shell. Standard input, output, and error streams stay attached to your terminal. `hd` returns the command exit code.

The tool stores a completion event in SQLite. The default database path is `$XDG_DATA_HOME/herdr-done/events.db`, or `$HOME/.local/share/herdr-done/events.db` when `XDG_DATA_HOME` is not set. Set `HD_DATABASE` to use another path.

`hd` does not store command output or environment values. It stores command arguments, working directory, start time, finish time, duration, result, read state, and available Herdr identifiers.

## Review events

```sh
hd list
hd list --unread
hd show ID
```

`hd list` shows recent events in completion order, newest first. `hd list --unread` shows only unread events. `hd show ID` prints one event. `hd` reports an error when no event has the requested ID.

## Interactive inbox

```sh
hd inbox
hd inbox --oneshot
```

The inbox opens without marking events as read. It refreshes the event list every second while it stays open. A pane jump keeps the inbox open and shows focus errors there. Use `hd inbox --oneshot` to close the inbox after a pane jump.

| Key | Action |
| --- | --- |
| `Down` or `j` | Select the next event |
| `Up` or `k` | Select the previous event |
| `Enter` | Show event details |
| `n` | Select the next unread event |
| `r` | Mark the selected event as read |
| `u` | Mark the selected event as unread |
| `J` | Focus the source Herdr pane and keep the live inbox open |
| `q` | Quit |

Read and unread changes persist in the database.

## Herdr integration

When Herdr context is available, `hd run` stores the session, workspace, tab, and pane identifiers. When Herdr is unavailable, the command still runs and the event has no Herdr identifiers.

```sh
hd jump ID
```

`hd jump` focuses the original pane through Herdr's documented `pane.focus` API. It does not attach to the pane terminal. The command needs `HERDR_ENV=1` and `HERDR_SOCKET_PATH`. If the pane no longer exists, `hd` reports the error and keeps the event.

## CLI reference

```text
Usage: hd run <COMMAND>...
Usage: hd list [OPTIONS]
Usage: hd show <ID>
Usage: hd inbox [OPTIONS]
Usage: hd jump <ID>
```
