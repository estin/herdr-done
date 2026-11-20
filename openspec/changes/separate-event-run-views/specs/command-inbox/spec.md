# Spec Delta

## MODIFIED Requirements

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
