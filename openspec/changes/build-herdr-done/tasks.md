# Tasks

## 1. Command Runner and Event Store

- [ ] 1.1 Add CLI parsing and `hd run -- COMMAND [ARGS...]`; verify direct execution, inherited streams, launch errors, and exit-code preservation with process tests
- [ ] 1.2 Add the event model and SQLite migrations, including running, completed, failed-to-start, and interrupted states; verify schema creation and event updates with temporary database tests
- [ ] 1.3 Persist command arguments, working directory, timestamps, duration, result, and available Herdr identifiers; verify that output and environment values are not stored
- [ ] 1.4 Add signal handling and storage-failure reporting; verify child termination behavior and confirm storage errors do not replace the child result
- [ ] 1.5 Add `hd list` and `hd show ID`; verify newest-first ordering, unread filtering, and unknown-ID errors

## 2. Interactive Inbox

- [ ] 2.1 Add the keyboard-driven inbox and event detail view; verify navigation and event fields with TUI tests
- [ ] 2.2 Add persistent read and unread actions and next-unread navigation; verify opening the inbox does not mark events read and next selects an unread event
- [ ] 2.3 Add focused user documentation for runner and inbox commands; verify examples match the CLI help

## 3. Herdr Context and Navigation

- [ ] 3.1 Read current pane context through the documented Herdr interface; verify context parsing against captured `herdr pane current` output and behavior when Herdr is unavailable
- [ ] 3.2 Add `hd jump ID` to focus an event's original pane through the documented `pane.focus` API without attaching; verify request parameters and stale-pane errors
- [ ] 3.3 Add end-to-end tests for commands started in separate panes and verify each completion retains its source pane identifiers
