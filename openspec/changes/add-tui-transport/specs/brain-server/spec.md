# brain-server Delta Spec

## ADDED Requirements

### Requirement: Interactive Turn Terminal Events
The server SHALL ensure that every interactive turn started through
`send_message()` or `send_message_stream()` ends with exactly one terminal
event delivered to the caller and published to subscribers:

- `TurnDone` for successful completion
- `Interrupted` for user-initiated cancellation
- `Error` for runtime failure

User-initiated cancellation SHALL NOT be surfaced as a generic error.

#### Scenario: Successful turn
- **WHEN** a turn completes normally
- **THEN** subscribers and streaming callers SHALL observe `TurnDone` as the terminal event

#### Scenario: Cancelled turn
- **WHEN** `cancel_turn(session_id)` is called for an active turn
- **THEN** subscribers and streaming callers SHALL observe `Interrupted` as the terminal event
- **AND** they SHALL NOT receive a generic cancellation `Error` instead

#### Scenario: Failed turn
- **WHEN** a turn fails due to provider or runtime error
- **THEN** subscribers and streaming callers SHALL observe `Error` as the terminal event

### Requirement: Attachable History and Live Events
Interactive clients SHALL be able to reconstruct session state by combining
message history APIs with the live event stream.

The server SHALL support the following flow without requiring a special attach
handshake:

1. load session metadata
2. load persisted message history
3. subscribe to live events
4. continue the session

#### Scenario: Attach to existing session
- **WHEN** a client attaches to a running server and opens an existing session
- **THEN** it SHALL be able to load history via session/message APIs and then receive live `ServerEvent` updates

#### Scenario: Per-session event ordering
- **WHEN** a session emits multiple events during a turn
- **THEN** subscribers SHALL observe those events in the same order they were produced for that session
