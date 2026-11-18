# Spec Delta

## Purpose

This capability records commands that users choose to run through `hd` and provides a persistent way to review their results. When Herdr context is available through a documented interface, events can link back to their source pane.

## ADDED Requirements

### Requirement: Run commands without shell interpretation
The system MUST run the supplied executable and arguments directly by default. It MUST inherit standard input, output, and error streams and return the child's exit result to the caller.

#### Scenario: Command succeeds
- **WHEN** a user runs a command that exits successfully through `hd run --`
- **THEN** the command uses inherited terminal streams and `hd` exits successfully

#### Scenario: Command exits with a non-zero code
- **WHEN** a user runs a command that exits with a non-zero code through `hd run --`
- **THEN** `hd` returns that exit code

#### Scenario: Executable cannot start
- **WHEN** the requested executable cannot be started
- **THEN** `hd` reports the launch failure and records a distinct failed-to-start event when storage is available

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
The system MUST let users list recent events and inspect an event by identifier. Lists MUST order events by finish time, newest first, and MUST support filtering for unread events.

#### Scenario: List recent events
- **WHEN** a user requests the event list
- **THEN** the system displays recent events in completion order with their result and duration

#### Scenario: Inspect an event
- **WHEN** a user requests an existing event by identifier
- **THEN** the system displays its command, working directory, result, timing, and available Herdr context

#### Scenario: Inspect an unknown event
- **WHEN** a user requests an identifier that does not exist
- **THEN** the system reports that no matching event exists

### Requirement: Review events in an interactive inbox
The system MUST provide a keyboard-driven inbox that supports event selection, event details, read and unread state, and navigation to the next unread event.

#### Scenario: Open inbox with unread events
- **WHEN** a user opens the inbox
- **THEN** the inbox shows unread state and recent events without marking them read solely because the inbox opened

#### Scenario: Mark an event read
- **WHEN** a user marks an unread event as read
- **THEN** that event no longer appears as unread

#### Scenario: Find the next unread event
- **WHEN** a user requests the next unread event
- **THEN** the inbox selects the next unread event rather than the next event of any state

### Requirement: Focus the source Herdr pane when supported
The system MUST use documented Herdr interfaces for context detection and pane navigation. The jump action MUST focus the source pane without attaching to its terminal. The system MUST retain event history when the source pane is no longer available and MUST report when navigation cannot be completed.

#### Scenario: Jump to an available source pane
- **WHEN** an event has Herdr pane context and the documented Herdr interface can resolve that pane
- **THEN** the system focuses the source pane without attaching to its terminal

#### Scenario: Source pane no longer exists
- **WHEN** an event's source pane cannot be resolved
- **THEN** the system reports that the pane is unavailable and retains the event
