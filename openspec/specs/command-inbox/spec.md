# command-inbox Specification

## Purpose

This capability records commands that users choose to run through `hd` and provides a persistent way to review their results. When Herdr context is available through a documented interface, events can link back to their source pane.

## Requirements

### Requirement: Run commands without shell interpretation
The system MUST run the supplied executable and arguments directly by default. It MUST inherit standard input, output, and error streams and return the child's exit result to the caller. The system MUST support opt-in completion sound and system notification alerts.

#### Scenario: Command succeeds
- **WHEN** a user runs a command that exits successfully through `hd run --`
- **THEN** the command uses inherited terminal streams and `hd` exits successfully

#### Scenario: Command exits with a non-zero code
- **WHEN** a user runs a command that exits with a non-zero code through `hd run --`
- **THEN** `hd` returns that exit code

#### Scenario: Executable cannot start
- **WHEN** the requested executable cannot be started
- **THEN** `hd` reports the launch failure and records a distinct failed-to-start event when storage is available

#### Scenario: Sound requested for completion
- **WHEN** a user runs `hd run --sound -- COMMAND` and the command completes
- **THEN** `hd` plays Herdr's configured completion sound without changing the command's exit result

#### Scenario: System notification requested for completion
- **WHEN** a user runs `hd run --notify -- COMMAND` and the command completes
- **THEN** `hd` sends a system notification with the command and its result without changing the command's exit result

#### Scenario: Alerts are not requested
- **WHEN** a user runs `hd run -- COMMAND` without alert options
- **THEN** `hd` does not play a sound or send a system notification

#### Scenario: Alert delivery fails
- **WHEN** a requested sound or system notification cannot be delivered
- **THEN** `hd` still returns the command's exit result

### Requirement: Create an event without running a command
The system MUST provide `hd event MESSAGE` to immediately create a completed event from a user-provided message without executing a process. The event MUST include the current working directory, invocation timestamp, and available Herdr context. The system MUST NOT display a success message by default. It MUST support an opt-in `--print-id` flag that prints the created event ID after successful storage.

#### Scenario: Create an event with a message
- **WHEN** a user runs `hd event "deployment is ready"`
- **THEN** the system stores a completed event with that message and returns without starting a process or writing success output

#### Scenario: Print the created event ID
- **WHEN** a user runs `hd event --print-id "deployment is ready"` and storage succeeds
- **THEN** the system prints the created event ID

#### Scenario: Event message is missing
- **WHEN** a user runs `hd event` without a message
- **THEN** the CLI reports invalid usage and does not create an event

#### Scenario: Event storage fails
- **WHEN** the system cannot persist an event created by `hd event`
- **THEN** it reports the storage error and returns a failure status

#### Scenario: Requested alert with quiet output
- **WHEN** a user runs `hd event --sound` or `hd event --notify` without `--print-id`
- **THEN** the system delivers the requested alert after storage and writes no success output to stdout

### Requirement: Send optional alerts for a manually created event
The system MUST support `--sound` and `--notify` on `hd event` and MUST leave both alerts disabled by default. Alert delivery failures MUST NOT remove the stored event or change a successful event command result.

#### Scenario: Sound alert is requested
- **WHEN** a user runs `hd event --sound MESSAGE`
- **THEN** the system creates the event and requests Herdr's configured completion sound

#### Scenario: System notification is requested
- **WHEN** a user runs `hd event --notify MESSAGE`
- **THEN** the system creates the event and sends a system notification with the event message

#### Scenario: Alerts are not requested
- **WHEN** a user runs `hd event MESSAGE` without alert options
- **THEN** the system creates the event without playing a sound or sending a notification

### Requirement: Persist command completion events
The system MUST store each tracked command's arguments, working directory, start time, finish time, duration, exit result, and available Herdr context in a local persistent event store. It MUST NOT store environment variables or command output by default.

#### Scenario: Command completes
- **WHEN** a tracked command finishes
- **THEN** its event is stored with a unique identifier and completion timestamp

#### Scenario: Herdr context is unavailable
- **WHEN** a command runs outside Herdr or Herdr context cannot be read
- **THEN** the command still runs and its event is stored without Herdr identifiers

#### Scenario: Storage fails after command completion
- **WHEN** the event store cannot save a completed command
- **THEN** `hd` reports the storage failure and preserves the command's exit result

### Requirement: List and inspect completion events
The system MUST let users list recent events and inspect an event by identifier. Lists MUST order events by finish time, newest first, and MUST support filtering for unread events. The inbox MUST display available Herdr workspace, tab, and pane names instead of their IDs. The inbox list MUST show the invocation time for both manually created events and command runs. The detail view for a manually created event MUST omit duration and finish time; the detail view for a command run MUST show its start time, finish time, and duration.

#### Scenario: List recent events
- **WHEN** a user requests the event list
- **THEN** the system displays recent events in completion order with their result, duration, invocation time, and available Herdr location names

#### Scenario: Invocation time in inbox list
- **WHEN** the inbox lists a manually created event or a command run
- **THEN** the row shows when the event or command was invoked, using the event start timestamp

#### Scenario: Inspect an event
- **WHEN** a user requests an existing event by identifier
- **THEN** the system displays its command, working directory, result, timing, and available Herdr context using names instead of IDs for workspace, tab, and pane labels

#### Scenario: Inspect a manually created event
- **WHEN** a user inspects an event created by `hd event`
- **THEN** the detail view shows its message, working directory, result, invocation time, and available Herdr context, without a duration or finish time

#### Scenario: Inspect a command run
- **WHEN** a user inspects an event created by `hd run`
- **THEN** the detail view shows its command, working directory, result, start time, finish time, duration, and available Herdr context

#### Scenario: Inspect an unknown event
- **WHEN** a user requests an identifier that does not exist
- **THEN** the system reports that no matching event exists

#### Scenario: Herdr names are unavailable
- **WHEN** an event has a Herdr ID but no corresponding saved name
- **THEN** the inbox does not present the ID as a human-readable name and shows the location as unavailable

### Requirement: Load layered inbox configuration
The system MUST load configuration in layers: built-in defaults, then an optional user configuration file, then environment variables with the `HD__` prefix and `__` between nested keys. Higher layers MUST override lower layers key by key. A missing configuration file MUST NOT be an error. Invalid configuration MUST produce an error that names the offending key and MUST prevent the command from continuing. The configuration MUST include a `database` key for the event store path and a `refresh_interval` key in human time syntax.

#### Scenario: Configuration file location
- **WHEN** the system looks for the user configuration file
- **THEN** it reads `$XDG_CONFIG_HOME/herdr-done/config.toml`, or `$HOME/.config/herdr-done/config.toml` when `XDG_CONFIG_HOME` is unset

#### Scenario: No configuration file exists
- **WHEN** a user opens the inbox without a configuration file or `HD__` variables
- **THEN** the inbox uses the built-in dark theme and the current default keybindings

#### Scenario: Configuration file overrides defaults
- **WHEN** the configuration file sets `theme = "light"` and binds `next_unread` to `["N"]`
- **THEN** the inbox uses the light theme colors, `N` selects the next unread event, and all unmentioned settings keep their defaults

#### Scenario: Color override on a built-in theme
- **WHEN** the configuration selects a built-in theme and sets an individual color key
- **THEN** that color replaces the theme value and the other theme colors stay unchanged

#### Scenario: Environment variable overrides the file
- **WHEN** the configuration file and an `HD__` environment variable set the same key
- **THEN** the environment variable value wins

#### Scenario: Invalid configuration value
- **WHEN** the configuration contains an unknown key, an unknown theme name, an invalid color, or an unparsable key binding
- **THEN** the system reports an error that names the offending key and does not open the inbox

#### Scenario: Database path from configuration
- **WHEN** the configuration sets `database` to a path
- **THEN** every `hd` command uses that path for the event store instead of the default location

#### Scenario: HD__DATABASE wins over the configuration file
- **WHEN** both `HD__DATABASE` and the `database` configuration key are set
- **THEN** the event store uses the `HD__DATABASE` path

#### Scenario: Default database path
- **WHEN** neither `HD__DATABASE` nor `database` is set
- **THEN** the event store stays at `$XDG_DATA_HOME/herdr-done/events.db`, or `$HOME/.local/share/herdr-done/events.db` when `XDG_DATA_HOME` is unset

#### Scenario: Refresh interval in human time
- **WHEN** the configuration sets `refresh_interval = "2s 500ms"`
- **THEN** the live inbox uses 2.5 seconds as its fallback refresh interval

#### Scenario: Invalid refresh interval
- **WHEN** `refresh_interval` is unparsable or zero
- **THEN** the system reports an error that names the `refresh_interval` key and does not open the inbox

### Requirement: Configure inbox theme colors
The system MUST ship built-in `dark` (default) and `light` themes and MUST allow individual color overrides on top of the selected theme. When a base16 scheme is configured, its mapped palette MUST replace the built-in theme colors before individual overrides apply. Color values MUST accept named colors and `#RRGGBB` hex strings. The configurable colors MUST cover the inbox title, unread marker, secondary text, success, failure, and running result labels, the inbox background, and the default text color. Built-in themes MUST leave background and default text unset so the terminal colors show through. The selection highlight MUST stay readable on any terminal background.

#### Scenario: Select a built-in theme
- **WHEN** the configuration sets `theme = "light"`
- **THEN** the inbox uses the light theme colors for every element the theme defines and keeps the terminal background

#### Scenario: Manual background override
- **WHEN** the configuration sets `[colors] background` or `[colors] text` without a base16 scheme
- **THEN** the inbox uses that value on top of the built-in theme

### Requirement: Load a base16 scheme as the inbox theme
The system MUST accept an optional `base16` configuration key naming a local tinted-theming scheme file in the spec-0.11 YAML format and MUST map its palette to the inbox colors. Individual `[colors]` overrides MUST still win over the scheme. The system MUST NOT fetch scheme files over the network. An unreadable or invalid scheme file MUST produce an error that names the `base16` key and MUST prevent the inbox from opening.

#### Scenario: Palette mapping
- **WHEN** the system maps a scheme palette to the inbox colors
- **THEN** title uses `base0D`, unread marker and running label use `base0A`, secondary text uses `base04`, success label uses `base0B`, failure label uses `base08`, background uses `base00`, and default text uses `base05`

#### Scenario: Scheme colors are applied
- **WHEN** the configuration sets `base16` to a readable spec-0.11 scheme file
- **THEN** the inbox renders with the mapped palette colors instead of the built-in theme colors

#### Scenario: Scheme background covers the inbox
- **WHEN** the inbox renders with a base16 scheme configured
- **THEN** the whole inbox area, including borders and empty space, uses the scheme background and default text colors

#### Scenario: Color override wins over the scheme
- **WHEN** the configuration sets `base16` and an individual `[colors]` key
- **THEN** that color replaces the mapped scheme value and the other mapped colors stay unchanged

#### Scenario: Scheme file is missing
- **WHEN** `base16` names a file that does not exist or cannot be read
- **THEN** the system reports an error that names the `base16` key and does not open the inbox

#### Scenario: Scheme file is invalid
- **WHEN** the scheme file is not valid spec-0.11 YAML, lacks a required palette entry, or contains an invalid hex color
- **THEN** the system reports an error that names the `base16` key and does not open the inbox

#### Scenario: No scheme configured
- **WHEN** the configuration does not set `base16`
- **THEN** theme resolution behaves exactly as before, with the built-in preset and `[colors]` overrides

### Requirement: Configure inbox keybindings
Each inbox action (down, up, inspect, back, next unread, mark read, mark unread, jump, clear read) MUST accept a list of key bindings, and the defaults MUST reproduce the current keys. Double `Ctrl-C` quit MUST remain fixed and MUST NOT be remappable.

#### Scenario: Quit binding cannot be remapped
- **WHEN** a user binds another key or action over the quit behavior
- **THEN** double `Ctrl-C` still quits and no configuration key can disable it

### Requirement: Review events in an interactive inbox
The system MUST provide a keyboard-driven inbox that supports event selection, event details, read and unread state, navigation to the next unread event, and clearing read events. By default, the inbox MUST refresh its recent events while it remains open, MUST wake up and refresh when the event store's write-ahead log changes, and MUST use the configured refresh interval as a fallback cadence. It MUST stay open after a pane jump. The system MUST support a one-shot mode that preserves the current behavior of closing the inbox after a pane jump. The inbox MUST apply the configured theme colors and MUST dispatch keys through the configured keybindings. The footer help MUST reflect the active keybindings. The inbox MUST exit only after two `Ctrl-C` key presses within two seconds. Pressing a back key (`q` or `Esc` by default) in the list view MUST keep the inbox open and show the quit notice. Pressing a back key in the detail view MUST return to the list view.

#### Scenario: Open inbox with unread events
- **WHEN** a user opens the inbox with unread events
- **THEN** the inbox shows unread state and recent events without marking them read solely because the inbox opened, and refreshes the event list while it remains open

#### Scenario: Refresh while viewing event details
- **GIVEN** the user keeps the inbox open in its list or detail view
- **WHEN** a new command completion event is stored
- **THEN** the inbox displays that event on its next refresh without losing the selected event when it remains available

#### Scenario: Mark an event read
- **WHEN** a user marks an unread event as read
- **THEN** that event no longer appears as unread

#### Scenario: Find the next unread event
- **WHEN** a user requests the next unread event
- **THEN** the inbox selects the next unread event rather than the next event of any state

#### Scenario: Clear read events from the inbox
- **WHEN** a user invokes the clear-read action in the inbox
- **THEN** the inbox removes all read events from the event store and refreshes the list without removing unread events

#### Scenario: Quit with two Ctrl-C presses
- **WHEN** a user presses `Ctrl-C` twice within two seconds from the list or detail view
- **THEN** the inbox exits

#### Scenario: First Ctrl-C shows quit notice
- **WHEN** a user presses `Ctrl-C` once without a second press within two seconds
- **THEN** the inbox stays open and shows "To quit, press Ctrl-C twice"

#### Scenario: q and Esc do not quit
- **WHEN** a user presses `q` or `Esc` from the list view
- **THEN** the inbox stays open and shows "To quit, press Ctrl-C twice"

#### Scenario: q and Esc in detail view return to list
- **WHEN** a user presses `q` or `Esc` from the detail view
- **THEN** the inbox returns to the list view and stays open

#### Scenario: Custom keybinding drives an action
- **WHEN** a user presses a key that the configuration binds to an inbox action
- **THEN** the inbox performs that action, and a default key that the configuration replaced no longer triggers it

#### Scenario: Footer reflects active bindings
- **WHEN** the inbox renders its footer help without a status message
- **THEN** the help lists the active keybindings for the inbox actions

#### Scenario: Theme colors are applied
- **WHEN** the inbox renders with a configured theme
- **THEN** the title, unread marker, secondary text, and result labels use the configured colors

#### Scenario: Jump from the live inbox
- **WHEN** a user selects the pane jump action in the live inbox
- **THEN** the system focuses the source pane and keeps the inbox open

#### Scenario: Jump from the one-shot inbox
- **WHEN** a user runs `hd inbox --oneshot` and selects the pane jump action
- **THEN** the system focuses the source pane and closes the inbox

#### Scenario: Jump fails in the live inbox
- **WHEN** a pane jump from the live inbox cannot complete
- **THEN** the inbox stays open and displays the navigation error

#### Scenario: WAL change wakes the inbox
- **GIVEN** a live inbox is open and idle between interval ticks
- **WHEN** another process stores a new event and the write-ahead log file changes
- **THEN** the inbox refreshes promptly without waiting for the next interval tick

#### Scenario: Interval fallback without WAL notifications
- **GIVEN** the write-ahead log produces no usable change notifications
- **WHEN** the configured refresh interval elapses
- **THEN** the inbox still refreshes its event list

#### Scenario: One-shot mode does not watch
- **WHEN** a user runs `hd inbox --oneshot`
- **THEN** the inbox behaves as before and closes after a pane jump

### Requirement: Navigate to the next unread completion
The system MUST provide `hd next` to open the interactive inbox with the next unread completion selected.

#### Scenario: An unread completion exists
- **WHEN** a user runs `hd next` and unread events exist
- **THEN** the inbox opens with an unread event selected and available for inspection

#### Scenario: No unread completion exists
- **WHEN** a user runs `hd next` and no unread events exist
- **THEN** the system reports that there are no unread events and does not open an empty inbox

### Requirement: Clear read completion events
The system MUST provide `hd clear` to permanently remove read completion events while retaining unread events.

#### Scenario: Read events exist
- **WHEN** a user runs `hd clear` and read events exist
- **THEN** the system removes all read events and reports the number removed

#### Scenario: No read events exist
- **WHEN** a user runs `hd clear` and no read events exist
- **THEN** the system reports that no read events were removed and retains all unread events

### Requirement: Focus the source Herdr pane when supported
The system MUST use documented Herdr interfaces for context detection and pane navigation. The jump action MUST focus the source pane without attaching to its terminal. The system MUST retain event history when the source pane is no longer available and MUST report when navigation cannot be completed.

#### Scenario: Jump to an available source pane
- **WHEN** an event has Herdr pane context and the documented Herdr interface can resolve that pane
- **THEN** the system focuses the source pane without attaching to its terminal

#### Scenario: Source pane no longer exists
- **WHEN** an event's source pane cannot be resolved
- **THEN** the system reports that the pane is unavailable and retains the event
