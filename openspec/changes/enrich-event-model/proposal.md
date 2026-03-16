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

2. **No tool approval / confirmation events**: if tool approval is ever added,
   the event model needs request/response events. Even without full approval,
   clients want to show "about to run shell command: ..." before execution.

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

### Enhanced Event enum

Extend the `Event` enum while keeping backward compatibility:

```rust
enum Event {
    // Existing (keep)
    Token { delta: String },
    ToolCallStart { id: String, name: String, arguments: String },
    ToolCallDone { id: String, result: String },
    MessageDone { message: Message },
    TurnDone { iterations: u32, total_tokens: Option<u64> },
    Error { code: ErrorCode, message: String, recoverable: bool },

    // New: streaming tool call args
    ToolCallDelta { id: String, name: String, arguments_delta: String },

    // New: tool approval flow
    ToolCallPending { id: String, name: String, arguments: String },
    ToolCallApproved { id: String },
    ToolCallRejected { id: String, reason: String },

    // New: progress
    Progress { phase: String, message: String, percent: Option<f32> },

    // New: session lifecycle
    SessionStart { session_id: Ulid },
    SessionResume { session_id: Ulid },

    // New: from harden-agent-loop
    Retry { attempt: u32, max: u32, error: String },
    Compaction { original_messages: usize, summary_tokens: usize },
    DoomLoopWarning { tool_name: String, repetitions: u32 },
}
```

### Transport rendering contract

The `Transport` trait's `send()` method receives these events. Each transport
decides how to render them:

- **CliTransport**: ignores most new events, keeps current simple behavior
- **TuiTransport** (future): renders deltas in split panes, shows progress bars
- **AcpTransport** (future): maps events to Agent Client Protocol messages

### InputEvent extensions

Extend `InputEvent` for richer client→engine communication:

```rust
enum InputEvent {
    Message(String),
    ToolApproval { id: String, approved: bool, reason: Option<String> },
    Cancel,
    SwitchSession(Ulid),
}
```

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
