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

## Deferred

- [ ] Update SimpleLoop to emit `ToolCallDelta` from provider stream (requires OpenAI provider to emit `ChatChunk::ToolCallDelta` instead of accumulating internally)
- [ ] Implement actual tool approval gate (wait for `InputEvent::ToolApproval` from transport)
- [x] Implement `InputEvent::Cancel` handling in Brain::run (requires CancellationToken plumbing)
- [x] Implement `InputEvent::SwitchSession` handling in Brain::run
- [ ] Write tests for InputEvent::Cancel and SwitchSession handling
