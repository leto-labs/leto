# brain-loops Delta Spec

## MODIFIED Requirements

### Requirement: Event Stream
The Event enum SHALL be extended with the following new variants while
preserving all existing variants:

- `ToolCallDelta { id: String, name: String, arguments_delta: String }` — incremental tool call argument fragment from the provider stream
- `ToolCallPending { id: String, name: String, arguments: serde_json::Value }` — tool call is assembled and ready for execution; emitted before execution begins
- `ToolCallApproved { id: String }` — tool call was approved (by user or auto-approval policy)
- `ToolCallRejected { id: String, reason: String }` — tool call was rejected
- `Progress { phase: String, message: String, percent: Option<f32> }` — progress update for long-running operations
- `SessionStart { session_id: Ulid }` — a new session has been created
- `SessionResume { session_id: Ulid }` — an existing session has been resumed
- `Retry { attempt: u32, max: u32, error: String }` — transient provider retry information
- `Compaction { original_messages: usize, summary_tokens: usize }` — context compaction summary
- `DoomLoopWarning { tool_name: String, repetitions: u32 }` — repeated-tool warning

The Event enum SHALL be marked `#[non_exhaustive]` to allow future additions without breaking downstream consumers.

#### Scenario: Streaming tool call deltas
- **WHEN** the provider streams tool call argument fragments
- **THEN** the event stream SHALL yield `ToolCallDelta` for each fragment
- **AND** after the tool call is assembled, the loop SHALL still emit the normal tool execution lifecycle

#### Scenario: Auto-approved tool lifecycle
- **WHEN** no approval policy is configured
- **THEN** `ToolCallPending` SHALL still be emitted
- **AND** `ToolCallApproved` SHALL be emitted immediately (auto-approved)
- **AND** execution SHALL proceed without waiting for transport input

#### Scenario: Session lifecycle events
- **WHEN** `Brain.run()` creates a new session
- **THEN** it SHALL emit `SessionStart` before the first turn
- **WHEN** `Brain.run()` resumes an existing session
- **THEN** it SHALL emit `SessionResume` before the first turn

#### Scenario: Robust loop emits hardening events
- **WHEN** retry, compaction, or repeated-tool mitigation occurs
- **THEN** the event stream SHALL emit the corresponding `Retry`, `Compaction`,
  or `DoomLoopWarning` event
