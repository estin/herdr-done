# Spec Delta

## MODIFIED Requirements

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
