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
hd run --sound -- just deploy
hd run --notify -- nix flake check
hd run --sound --notify -- cargo build --release
```

`--sound` plays Herdr's configured `done` sound after the command finishes. It needs Herdr to be available. `--notify` sends a system notification with the command and its result. Both flags are off by default and can be used together or separately. Alert errors are reported to stderr. They do not change the command exit code. Launch failures do not trigger alerts.

`hd run --` passes the executable and arguments directly. The command does not use a shell. Standard input, output, and error streams stay attached to your terminal. `hd` returns the command exit code.

The tool stores a completion event in SQLite. The default database path is `$XDG_DATA_HOME/herdr-done/events.db`, or `$HOME/.local/share/herdr-done/events.db` when `XDG_DATA_HOME` is not set. Set `HD_DATABASE` to use another path.

`hd` does not store command output or environment values. It stores command arguments, working directory, start time, finish time, duration, result, read state, and available Herdr identifiers.

## Record an event

```sh
hd event "deployment is ready"
hd event --sound --notify "deployment is ready"
```

`hd event MESSAGE` stores a completed event from a message without starting a process. The message appears in event lists and details. The event includes the current working directory, timestamps, duration, and available Herdr context. `--sound` and `--notify` use the same alert behavior as `hd run`. They are off by default. Alerts are sent only after the event is saved.

A missing message is invalid usage. A storage failure returns a failure status and sends no alerts.

## Review events

```sh
hd list
hd list --unread
hd show ID
hd next
hd clear
```

`hd list` shows recent events in completion order, newest first. `hd list --unread` shows only unread events. `hd show ID` prints one event. `hd` reports an error when no event has the requested ID.

`hd next` opens the inbox with the newest unread event selected. If no unread event exists, `hd next` reports `no unread events` and does not open the inbox. `hd clear` permanently removes read events and keeps unread events. It prints `cleared N read events` or `no read events were removed`.

## Interactive inbox

```sh
hd inbox
hd inbox --oneshot
```

The inbox opens without marking events as read. It refreshes the event list every second while it stays open. A pane jump keeps the inbox open and shows focus errors there. Press Ctrl-C twice within two seconds to exit. In event details, `q` and `Esc` return to the list. In the list, they keep the inbox open and show "To quit, press Ctrl-C twice". Use `hd inbox --oneshot` to close the inbox after a pane jump.

| Key | Action |
| --- | --- |
| `Down` or `j` | Select the next event |
| `Up` or `k` | Select the previous event |
| `Enter` | Show event details |
| `n` | Select the next unread event |
| `r` | Mark the selected event as read |
| `u` | Mark the selected event as unread |
| `c` | Clear read events |
| `J` | Focus the source Herdr pane and keep the live inbox open |
| `Ctrl-C` twice | Quit the inbox |
| `q` or `Esc` | Return to the list from details, or show the quit notice in the list |

Read and unread changes persist in the database.

## Herdr integration

When Herdr context is available, `hd run` stores the session, workspace, tab, and pane identifiers. When Herdr is unavailable, the command still runs and the event has no Herdr identifiers.

```sh
hd jump ID
```

`hd jump` focuses the original pane through Herdr's documented `pane.focus` API. It does not attach to the pane terminal. The command needs `HERDR_ENV=1` and `HERDR_SOCKET_PATH`. If the pane no longer exists, `hd` reports the error and keeps the event.

## CLI reference

```text
Usage: hd run [OPTIONS] <COMMAND>...
Usage: hd event [OPTIONS] <MESSAGE>
Usage: hd list [OPTIONS]
Usage: hd show <ID>
Usage: hd inbox [OPTIONS]
Usage: hd next
Usage: hd clear
Usage: hd jump <ID>
```
