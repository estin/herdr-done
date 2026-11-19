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
The system MUST let users list recent events and inspect an event by identifier. Lists MUST order events by finish time, newest first, and MUST support filtering for unread events. The inbox MUST display available Herdr workspace, tab, and pane names instead of their IDs.

#### Scenario: List recent events
- **WHEN** a user requests the event list
- **THEN** the system displays recent events in completion order with their result, duration, and available Herdr location names

#### Scenario: Inspect an event
- **WHEN** a user requests an existing event by identifier
- **THEN** the system displays its command, working directory, result, timing, and available Herdr context using names instead of IDs for workspace, tab, and pane labels

#### Scenario: Inspect an unknown event
- **WHEN** a user requests an identifier that does not exist
- **THEN** the system reports that no matching event exists

#### Scenario: Herdr names are unavailable
- **WHEN** an event has a Herdr ID but no corresponding saved name
- **THEN** the inbox does not present the ID as a human-readable name and shows the location as unavailable

### Requirement: Review events in an interactive inbox
The system MUST provide a keyboard-driven inbox that supports event selection, event details, read and unread state, navigation to the next unread event, and clearing read events. By default, the inbox MUST refresh its recent events while it remains open and MUST stay open after a pane jump. The system MUST support a one-shot mode that preserves the current behavior of closing the inbox after a pane jump. The inbox MUST exit only after two `Ctrl-C` key presses within two seconds. Pressing `q` or `Esc` MUST keep the inbox open and show the quit notice.

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
- **WHEN** a user presses `q` or `Esc` from the list or detail view
- **THEN** the inbox stays open and shows "To quit, press Ctrl-C twice"

#### Scenario: Jump from the live inbox
- **WHEN** a user selects the pane jump action in the live inbox
- **THEN** the system focuses the source pane and keeps the inbox open

#### Scenario: Jump from the one-shot inbox
- **WHEN** a user runs `hd inbox --oneshot` and selects the pane jump action
- **THEN** the system focuses the source pane and closes the inbox

#### Scenario: Jump fails in the live inbox
- **WHEN** a pane jump from the live inbox cannot complete
- **THEN** the inbox stays open and displays the navigation error

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
