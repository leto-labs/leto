# brain-loops Delta Spec

## MODIFIED Requirements

### Requirement: Event Stream
The Event enum SHALL be extended with the following new variants while preserving all existing variants:

- `ToolCallDelta { id: String, name: String, arguments_delta: String }` — incremental tool call argument fragment from the provider stream
- `ToolCallPending { id: String, name: String, arguments: String }` — tool call is assembled and ready for execution; emitted before execution begins
- `ToolCallApproved { id: String }` — tool call was approved (by user or auto-approval policy)
- `ToolCallRejected { id: String, reason: String }` — tool call was rejected
- `Progress { phase: String, message: String, percent: Option<f32> }` — progress update for long-running operations
- `SessionStart { session_id: Ulid }` — a new session has been created
- `SessionResume { session_id: Ulid }` — an existing session has been resumed

The Event enum SHALL be marked `#[non_exhaustive]` to allow future additions without breaking downstream consumers.

#### Scenario: Streaming tool call deltas
- **WHEN** the provider streams tool call argument fragments
- **THEN** the event stream SHALL yield `ToolCallDelta` for each fragment
- **AND** after all fragments are assembled, yield `ToolCallPending` with the complete arguments
- **AND** then `ToolCallStart` when execution begins (preserving existing contract)

#### Scenario: Tool approval flow
- **WHEN** a tool call requires approval (configured via policy)
- **THEN** the event stream SHALL yield `ToolCallPending`
- **AND** wait for `InputEvent::ToolApproval` from the transport
- **AND** yield `ToolCallApproved` or `ToolCallRejected` based on the response
- **AND** only proceed with execution if approved

#### Scenario: Auto-approval (default)
- **WHEN** no approval policy is configured
- **THEN** `ToolCallPending` SHALL still be emitted
- **AND** `ToolCallApproved` SHALL be emitted immediately (auto-approved)
- **AND** execution SHALL proceed without waiting for transport input

#### Scenario: Session lifecycle events
- **WHEN** `Brain.run()` creates a new session
- **THEN** it SHALL emit `SessionStart` before the first turn
- **WHEN** `Brain.run()` resumes an existing session
- **THEN** it SHALL emit `SessionResume` before the first turn


## MODIFIED Requirements

### Requirement: Event Stream
The Event enum SHALL be extended with the following new variants while preserving all existing variants:

- `ToolCallDelta { id: String, name: String, arguments_delta: String }` — incremental tool call argument fragment from the provider stream
- `ToolCallPending { id: String, name: String, arguments: String }` — tool call is assembled and ready for execution; emitted before execution begins
- `ToolCallApproved { id: String }` — tool call was approved (by user or auto-approval policy)
- `ToolCallRejected { id: String, reason: String }` — tool call was rejected
- `Progress { phase: String, message: String, percent: Option<f32> }` — progress update for long-running operations
- `SessionStart { session_id: Ulid }` — a new session has been created
- `SessionResume { session_id: Ulid }` — an existing session has been resumed

The Event enum SHALL be marked `#[non_exhaustive]` to allow future additions without breaking downstream consumers.

#### Scenario: Streaming tool call deltas
- **WHEN** the provider streams tool call argument fragments
- **THEN** the event stream SHALL yield `ToolCallDelta` for each fragment
- **AND** after all fragments are assembled, yield `ToolCallPending` with the complete arguments
- **AND** then `ToolCallStart` when execution begins (preserving existing contract)

#### Scenario: Tool approval flow
- **WHEN** a tool call requires approval (configured via policy)
- **THEN** the event stream SHALL yield `ToolCallPending`
- **AND** wait for `InputEvent::ToolApproval` from the transport
- **AND** yield `ToolCallApproved` or `ToolCallRejected` based on the response
- **AND** only proceed with execution if approved

#### Scenario: Auto-approval (default)
- **WHEN** no approval policy is configured
- **THEN** `ToolCallPending` SHALL still be emitted
- **AND** `ToolCallApproved` SHALL be emitted immediately (auto-approved)
- **AND** execution SHALL proceed without waiting for transport input

#### Scenario: Session lifecycle events
- **WHEN** `Brain.run()` creates a new session
- **THEN** it SHALL emit `SessionStart` before the first turn
- **WHEN** `Brain.run()` resumes an existing session
- **THEN** it SHALL emit `SessionResume` before the first turn
