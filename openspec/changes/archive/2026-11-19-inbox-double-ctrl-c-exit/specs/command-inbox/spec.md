# Spec Delta

## MODIFIED Requirements

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
