# Spec Delta

## MODIFIED Requirements

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
