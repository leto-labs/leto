# brain-transports Delta Spec

## MODIFIED Requirements

### Requirement: InputEvent Enum
The `InputEvent` enum SHALL be extended with new variants:

- `ToolApproval { id: String, approved: bool, reason: Option<String> }` — user's response to a tool approval request
- `Cancel` — user requests cancellation of the current turn
- `SwitchSession(Ulid)` — user requests switching to a different session

The InputEvent enum SHALL be marked `#[non_exhaustive]`.

#### Scenario: Tool approval input
- **WHEN** a transport receives a tool approval decision from the user
- **THEN** it SHALL produce `InputEvent::ToolApproval { id, approved, reason }`

#### Scenario: Cancel input
- **WHEN** the user requests cancellation (e.g., Ctrl+C in CLI, cancel button in UI)
- **THEN** the transport SHALL produce `InputEvent::Cancel`

#### Scenario: Session switch
- **WHEN** the user selects a different session
- **THEN** the transport SHALL produce `InputEvent::SwitchSession(session_id)`

### Requirement: Transport Trait
The Transport trait SHALL remain unchanged. New event variants are handled by
existing `send()` and `recv()` methods. Transport implementations that do not
recognize new variants SHOULD ignore them gracefully.

#### Scenario: Unknown event variant
- **WHEN** a transport receives an Event variant it does not handle
- **THEN** it SHALL ignore it without error (log at debug level)

### Requirement: CliTransport
CliTransport SHALL be updated to handle new events:
- `ToolCallDelta` — optionally print argument fragments (configurable verbosity)
- `ToolCallPending` — print tool name and args before execution
- `Progress` — print progress message
- `SessionStart` / `SessionResume` — print session info
- All other new variants — ignore silently

#### Scenario: Verbose CLI mode
- **WHEN** CliTransport is configured with verbose mode
- **THEN** it SHALL print ToolCallDelta fragments and Progress events

#### Scenario: Default CLI mode
- **WHEN** CliTransport uses default settings
- **THEN** it SHALL print ToolCallPending and ToolCallDone but skip deltas


## MODIFIED Requirements

### Requirement: InputEvent Enum
The `InputEvent` enum SHALL be extended with new variants:

- `ToolApproval { id: String, approved: bool, reason: Option<String> }` — user's response to a tool approval request
- `Cancel` — user requests cancellation of the current turn
- `SwitchSession(Ulid)` — user requests switching to a different session

The InputEvent enum SHALL be marked `#[non_exhaustive]`.

#### Scenario: Tool approval input
- **WHEN** a transport receives a tool approval decision from the user
- **THEN** it SHALL produce `InputEvent::ToolApproval { id, approved, reason }`

#### Scenario: Cancel input
- **WHEN** the user requests cancellation (e.g., Ctrl+C in CLI, cancel button in UI)
- **THEN** the transport SHALL produce `InputEvent::Cancel`

#### Scenario: Session switch
- **WHEN** the user selects a different session
- **THEN** the transport SHALL produce `InputEvent::SwitchSession(session_id)`

### Requirement: Transport Trait
The Transport trait SHALL remain unchanged. New event variants are handled by
existing `send()` and `recv()` methods. Transport implementations that do not
recognize new variants SHOULD ignore them gracefully.

#### Scenario: Unknown event variant
- **WHEN** a transport receives an Event variant it does not handle
- **THEN** it SHALL ignore it without error (log at debug level)

### Requirement: CliTransport
CliTransport SHALL be updated to handle new events:
- `ToolCallDelta` — optionally print argument fragments (configurable verbosity)
- `ToolCallPending` — print tool name and args before execution
- `Progress` — print progress message
- `SessionStart` / `SessionResume` — print session info
- All other new variants — ignore silently

#### Scenario: Verbose CLI mode
- **WHEN** CliTransport is configured with verbose mode
- **THEN** it SHALL print ToolCallDelta fragments and Progress events

#### Scenario: Default CLI mode
- **WHEN** CliTransport uses default settings
- **THEN** it SHALL print ToolCallPending and ToolCallDone but skip deltas
