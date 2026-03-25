# Control Flow

This page explains how one runtime session moves from queued input to visible boundaries.

If the runtime manual has one section to internalize, it is this one.

## The Core Rule

The engine runs work in this order:

1. begin a turn if queued input is ready
2. apply safe-boundary runtime injections
3. drain required pending runtime actions
4. ask the loop for ordinary policy
5. execute the chosen step

That order lives in [`engine.rs`](../../../crates/agent-runtime/src/engine.rs).

## The Main Drive Loop

[`engine.rs`](../../../crates/agent-runtime/src/engine.rs) makes the control flow explicit:

```rust
if self.apply_queued_steering().await? { continue; }
if self.apply_pending_direct_messages().await? { continue; }
if self.apply_pending_child_reports().await? { continue; }
if self.apply_pending_promoted_pty_events().await? { continue; }

let queued_action = snapshot.pending_runtime_actions.front().cloned();
let (decision, should_consume_pending_decision) = if let Some(action) = queued_action {
    (action, true)
} else {
    (self.strategy.decide(...).await?, false)
};
```

This is the shortest honest summary of the current runtime architecture:

- safe-boundary injections come first
- queued required mutations come second
- loop policy comes third

## Turn Lifecycle

```mermaid
sequenceDiagram
    participant Client as caller / outer surface
    participant Runtime as session engine
    participant Loop as loop strategy
    participant Provider as provider
    participant Tool as tool executor

    Client->>Runtime: submit input or control
    Runtime->>Runtime: begin turn if needed
    Runtime->>Runtime: apply safe-boundary injections
    Runtime->>Runtime: drain required pending runtime actions
    Runtime->>Loop: ask for next policy decision if no queued action
    Loop-->>Runtime: LoopDecision
    Runtime->>Provider: run provider step when requested
    Provider-->>Runtime: assistant output / tool calls / usage
    Runtime->>Tool: execute tool batch if needed
    Tool-->>Runtime: tool result
    Runtime-->>Client: runtime events + visible boundary
```

## Mutations vs Reads

This is the most important distinction to keep straight.

### Direct Reads

Direct reads execute immediately because they do not change the live control state.

Examples:

- list visible agents
- read agent mail
- list PTYs
- fetch a PTY snapshot
- read delivered PTY events

Analogy:

- a direct read is checking a gauge

### Required Runtime Actions

Mutating commands are converted into `pending_runtime_actions` and drained by the engine.

Examples:

- spawn agent
- send agent input
- pause or interrupt another runtime
- open, write, execute, resize, or close a PTY

Analogy:

- a required runtime action is moving a control lever

The caller can request it, but the runtime still sequences it through the canonical execution path.

## Why External Mutations Still Flow Through The Loop Path

This is easy to misunderstand.

The loop does **not** get discretionary veto over an explicit external mutation request.

Instead:

- external mutation requests become required pending runtime actions
- the runtime drains them before normal loop policy
- the loop path remains the single execution path for state-changing work

That gives one understandable model:

- reads stay direct
- mutations are scheduled

## Direct Reads vs Queued Mutations

### Session Command Matrix

| Surface | Example | Direct or queued? | Why |
| --- | --- | --- | --- |
| `SessionCommand::SubmitInput` | new user/developer input | direct enqueue into session input buffers | it is raw session input, not a loop action |
| `SessionCommand::Control` | pause, resume, steer, interrupt | direct queue into control state | control signals are runtime-owned |
| `SessionCommand::Approve` | tool approval decision | direct | it resolves an existing gate |
| `SessionCommand::Agent` read | `ReadAgentMessages`, `ListAgents` | direct | query only |
| `SessionCommand::Agent` mutation | `SpawnAgent`, `SendAgentInput`, `InterruptAgent` | queued as required runtime action | changes live runtime state |
| `SessionCommand::Pty` read | `ListPtys`, `GetPty`, `ReadPtyEvents` | direct | query only |
| `SessionCommand::Pty` mutation | `OpenPty`, `ExecutePtyBatch`, `ClosePty` | queued as required runtime action | changes live PTY/runtime state |

## Safe Boundaries

A safe boundary is a point where the runtime is allowed to surface runtime-originated facts back into the transcript.

Examples:

- a direct child message with `Direct` delivery
- a semantic child report
- a PTY subscription event with `PromoteToDeveloper`
- queued steering

Why not inject them immediately?

Because mid-phase transcript mutation is hard to reason about. The runtime uses boundaries to keep the session legible:

- provider work should not be surprised by transcript changes mid-stream
- tool execution should not race with model-visible message injection
- the transcript should change in a way users can reason about later

See [transcript-and-boundaries.md](transcript-and-boundaries.md).

## Loop Policy Starts After Required Work

Once boundary work and required actions are exhausted, the engine asks the loop for normal policy.

With [`SimpleLoop`](../../../crates/agent-loops/src/simple.rs), that policy is intentionally small:

- request approval if needed
- wait for blocking children
- execute pending tools
- finish completed turns
- queue doom-loop steering
- compact when advised
- otherwise run provider
- otherwise wait for input

This is what "small loop over rich runtime" means in practice.

## Waits And Holds

Waiting is not the same as pausing forever.

`WaitRequest` lets the runtime hold progress while watching one or more children. If the budget expires, the runtime can:

- release the hold and leave the child alive
- or interrupt the child before releasing the hold

That distinction exists so "blocking for completion" and "backgrounding while still alive" remain different runtime states rather than one overloaded behavior.

See [subagents.md](subagents.md).

## Worked Example: Spawn Then Wait

1. caller requests `SpawnAgent`
2. runtime converts it into a required pending runtime action
3. engine drains it and executes `LoopDecision::SpawnAgent`
4. child runtime is created and registered
5. if the request also carries a wait, runtime enters child-wait behavior
6. when the child finishes or times out, runtime updates child state and boundary

The important part is that the runtime still owns the mechanics:

- registry registration
- event watching
- wait behavior
- transcript report injection

The loop does not implement those from scratch.

## Worked Example: Open PTY Then Promote Output

1. caller requests `OpenPty`
2. runtime queues a required PTY action
3. engine drains it and opens the PTY
4. runtime subscribes to PTY events if requested
5. delivered PTY events go into runtime event buffers
6. only subscribed `PromoteToDeveloper` events are later rendered into transcript at a safe boundary

That is how the runtime keeps "observer event", "subscription delivery", and "transcript-visible message" separate.

## Related Reading

- [state.md](state.md)
- [decisions-and-events.md](decisions-and-events.md)
- [subagents.md](subagents.md)
- [pty.md](pty.md)
- [transcript-and-boundaries.md](transcript-and-boundaries.md)
