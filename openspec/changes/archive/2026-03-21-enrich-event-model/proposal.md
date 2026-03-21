# Proposal: enrich-event-model

## Why

brain's current `Event` enum was designed for a simple CLI that prints tokens
to stdout. To power rich clients (TUI with ratatui, VSCode via Agent Client
Protocol, web UIs), the event model needs to carry more structured information.

Current gaps:

1. **No streaming tool call arguments**: the model streams tool call arguments
   incrementally (delta by delta), but brain buffers them internally and only
   emits `ToolCallStart` (with complete args) and `ToolCallDone`. A TUI wants
   to show the arguments being assembled in real-time (like Cursor does).

2. **No tool lifecycle staging**: clients need to show that a tool call is
   pending and then auto-approved before execution, even before a future
   transport-driven approval gate exists.

3. **No progress / status events**: long-running operations (model loading,
   MCP server startup, compaction) have no way to signal progress.

4. **No structured content**: `Token { delta: String }` assumes text-only
   responses. Models increasingly return structured content (code blocks,
   images, citations). The event model should support rich content.

5. **No session lifecycle events**: clients need to know when a session starts,
   resumes, or ends — not just individual turns.

6. **Agent Client Protocol alignment**: the Agent Client Protocol defines a
   specific event model. brain's events should map cleanly to ACP messages
   without lossy translation.

## What

This change adds the implemented event/input model enrichment while keeping the
runtime contract additive and backward-compatible.

### Implemented event additions

Extend the `Event` enum with:

```rust
enum Event {
    // Existing (keep)
    Token { delta: String },
    ToolCallStart { id: String, name: String, arguments: serde_json::Value },
    ToolCallDone { id: String, result: String },
    MessageDone { message: Message },
    TurnDone { iterations: u32, total_tokens: u32 },
    Error { code: ErrorCode, message: String, recoverable: bool },

    // New: streaming tool call args
    ToolCallDelta { id: String, name: String, arguments_delta: String },

    // New: tool lifecycle
    ToolCallPending { id: String, name: String, arguments: serde_json::Value },
    ToolCallApproved { id: String },
    ToolCallRejected { id: String, reason: String },

    // New: progress
    Progress { phase: String, message: String, percent: Option<f32> },

    // New: session lifecycle
    SessionStart { session_id: Ulid },
    SessionResume { session_id: Ulid },

    // New: runtime hardening
    Retry { attempt: u32, max: u32, error: String },
    Compaction { original_messages: usize, summary_tokens: usize },
    DoomLoopWarning { tool_name: String, repetitions: u32 },
}
```

### Implemented input additions

Extend `InputEvent` with:

```rust
enum InputEvent {
    Message(String),
    ToolApproval { id: String, approved: bool, reason: Option<String> },
    Cancel,
    SwitchSession(Ulid),
}
```

### Implemented supporting changes

- mark `Event`, `InputEvent`, and `ChatChunk` as `#[non_exhaustive]`
- add `ChatChunk::ToolCallDelta`
- update `SimpleLoop` to forward provider-emitted tool-call deltas
- update `SimpleLoop` to emit `ToolCallPending` then `ToolCallApproved` before
  execution
- update `Brain::run()` to emit `SessionStart` and handle `Cancel` /
  `SwitchSession`
- update `CliTransport` to handle the additive events without breaking the
  simple CLI flow

### Explicit follow-up work

This change does not implement a transport-driven approval gate. `ToolApproval`
is added to the input model now so future transports can use it without
another breaking event-model expansion.

## Change Dependencies

- **Requires**: none (independent, can start immediately)
- **Required by**: `harden-agent-loop` (uses Retry, Compaction, DoomLoopWarning event variants)
- **Required by**: `add-tui-transport` (TUI renders all new event variants)
- **Required by**: `add-server-architecture` (ServerEvent wraps Event)

## Impact

- **Modifies**: `agent-loop` spec (Event enum), `transport` spec (InputEvent)
- **Modifies**: `brain-types` (Event, InputEvent enums)
- **Modifies**: `brain-loops` (emit new events)
- **Modifies**: `brain-transports` (handle new events in CliTransport)
- **No breaking changes**: new enum variants are additive; existing pattern matches
  need a wildcard arm (which is standard Rust practice for non-exhaustive enums)
