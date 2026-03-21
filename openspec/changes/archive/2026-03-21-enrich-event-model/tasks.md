# Tasks: enrich-event-model

## Implementation Checklist

- [x] Add `ToolCallDelta` variant to Event enum
- [x] Add `ToolCallPending`, `ToolCallApproved`, `ToolCallRejected` variants to Event
- [x] Add `Progress` variant to Event
- [x] Add `SessionStart`, `SessionResume` variants to Event
- [x] Add `Retry`, `Compaction`, `DoomLoopWarning` variants (coordinated with harden-agent-loop)
- [x] Add `ToolApproval`, `Cancel`, `SwitchSession` variants to InputEvent
- [x] Mark Event and InputEvent as `#[non_exhaustive]` to allow future additions
- [x] Add `ChatChunk::ToolCallDelta` variant to stream types (enables providers to emit deltas)
- [x] Mark `ChatChunk` as `#[non_exhaustive]`
- [x] Update SimpleLoop to forward `ChatChunk::ToolCallDelta` as `Event::ToolCallDelta`
- [x] Update SimpleLoop to emit `ToolCallPending` + `ToolCallApproved` (auto-approve) before tool execution
- [x] Update Brain::run to emit `SessionStart` event
- [x] Update Brain::run to handle new `InputEvent` variants (skip with `Some(_) => continue`)
- [x] Update CliTransport to handle new events (render pending, progress, session; wildcard for rest)
- [x] Fix `#[non_exhaustive]` fallout in examples and provider tests (wildcard arms)
- [x] Ensure backward compatibility: all 119 existing tests pass unchanged
- [x] Write test: `tool_call_emits_pending_and_approved` (SimpleLoop emits Pending→Approved→Start order)
- [x] Write test: `run_emits_session_start` (Brain::run first event is SessionStart)

## Follow-up Work

The following work remains outside the archived scope of this additive event
model pass:

- implement a transport-driven tool approval gate
- expand provider-side tool-call delta coverage where providers still buffer
  arguments internally
- add more focused tests around `InputEvent::Cancel` and `SwitchSession`
