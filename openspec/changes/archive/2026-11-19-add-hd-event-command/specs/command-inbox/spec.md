# Spec Delta

## ADDED Requirements

### Requirement: Create an event without running a command
The system MUST provide `hd event MESSAGE` to immediately create a completed event from a user-provided message without executing a process. The event MUST include the current working directory, timestamps, duration, and available Herdr context.

#### Scenario: Create an event with a message
- **WHEN** a user runs `hd event "deployment is ready"`
- **THEN** the system stores a completed event with that message and returns without starting another process

#### Scenario: Event message is missing
- **WHEN** a user runs `hd event` without a message
- **THEN** the CLI reports invalid usage and does not create an event

#### Scenario: Event storage fails
- **WHEN** the system cannot persist an event created by `hd event`
- **THEN** it reports the storage error and returns a failure status

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
