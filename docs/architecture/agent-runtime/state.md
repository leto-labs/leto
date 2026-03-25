# State Model

This page describes the in-memory state the runtime owns for one session.

The central type is [`SessionState`](../../../crates/agent-runtime/src/session.rs).

## Why The Runtime Needs Rich State

The runtime is not just keeping a transcript. It is keeping the entire **live control plane** for one agent session.

That includes:

- model-visible transcript
- control queues
- child runtime snapshots
- PTY snapshots and subscriptions
- advisory signals like context pressure
- visible phase and boundary

Analogy:

- the transcript is the conversation log
- `SessionState` is the full dashboard, scheduler, and staging area

## Key State Buckets

[`session.rs`](../../../crates/agent-runtime/src/session.rs) groups runtime state into a few major categories.

```rust
pub struct SessionState {
    pub children: BTreeMap<RuntimeId, ChildRuntimeState>,
    pub mailbox: VecDeque<AgentMessage>,
    pub pending_direct_messages: Vec<AgentMessage>,
    pub pending_child_reports: Vec<ChildReport>,
    pub transcript: Vec<Message>,
    pub recent_operations: VecDeque<RuntimeOperationResult>,
    pub pending_runtime_actions: VecDeque<LoopDecision>,
    pub ptys: BTreeMap<PtyId, PtySessionState>,
    pub pty_subscriptions: Vec<PtySubscription>,
    pub recent_pty_events: VecDeque<DeliveredPtyEvent>,
    pub pending_promoted_pty_events: Vec<DeliveredPtyEvent>,
    // ...
}
```

Read the full definition in [`session.rs`](../../../crates/agent-runtime/src/session.rs).

## SessionState Field Groups

| Group | Fields | What it means in practice |
| --- | --- | --- |
| Identity | `session_id`, `parent` | Which runtime this is, and whether it is a child of another runtime |
| Child state | `children` | Snapshots of direct child runtimes by stable id |
| Message transport | `inbox`, `mailbox`, `pending_direct_messages`, `pending_child_reports` | Routed envelopes, stored mail, and transcript-boundary injections |
| Transcript | `transcript` | Canonical provider-visible conversation state |
| Runtime operation history | `recent_operations`, `last_subcall`, `last_transcript_rewrite`, `last_transcript_append` | Bounded results exposed back to loop strategies |
| Required mutation queue | `pending_runtime_actions` | Queued state-changing runtime actions waiting to be drained |
| Advisories | `context_pressure`, `doom_loop` | Runtime-generated signals that influence loop policy |
| PTY state | `ptys`, `pty_subscriptions`, `recent_pty_events`, `pending_promoted_pty_events`, `last_pty_capture` | Managed terminal sessions and their event state |
| Turn/phase state | `phase`, `boundary`, `active_turn`, `turn_index`, `iteration_count` | Where the runtime is in the current execution lifecycle |
| Turn staging | `pending_inputs`, `pending_steering`, `pending_tool_calls`, `active_tool_call`, `pending_approval`, `active_wait`, `pending_interrupt` | Work that is staged, gated, or actively in flight |
| Completion | `pending_completion`, `last_finish_reason` | Whether the turn is ready to finish and why provider work last stopped |

## Phase vs Boundary

These two concepts are related but different.

### Phase

`SessionPhase` answers:

- what kind of work is currently happening?

Examples:

- `Idle`
- `RunningProvider`
- `RunningTool`
- `AwaitingApproval`
- `AwaitingChildren`
- `Paused`

### Boundary

`SessionBoundary` answers:

- what externally visible stable point has the runtime reached?

Examples:

- `AwaitingInput`
- `AwaitingApproval`
- `AwaitingChildren`
- `Paused`
- `TurnFinished`

Do not confuse:

- "the runtime is running a provider call" with
- "the runtime is at a boundary where transcript injections are allowed"

## Children Are Snapshots, Not Embedded Engines

`children` stores `ChildRuntimeState` snapshots, not nested full engines.

Why that matters:

- the real child runtime is managed elsewhere in the registry
- local `SessionState` only keeps the snapshot the parent needs to reason about

Analogy:

- the registry owns the live remote worker
- `SessionState` keeps the parent's dashboard card for that worker

See [subagents.md](subagents.md).

## PTYs Are Managed State Too

`ptys` stores `PtySessionState` snapshots that the current runtime knows about through:

- ownership
- explicit subscription

That is why PTYs feel closer to child runtimes than to one-shot tools.

The runtime needs to remember:

- PTY id
- owner runtime id
- working directory
- rows/cols
- status
- output cursor
- exit code when known

See [pty.md](pty.md).

## Pending Runtime Actions

`pending_runtime_actions` is the runtime's queue of required state-changing work.

This queue exists so the architecture has one understandable mutation path:

- external state-changing requests are recorded here
- engine drains them before normal loop policy

Do not read this queue as "optional suggestions for the loop." They are required actions waiting for execution.

## Recent Operations

`recent_operations` and the `last_*` fields exist so loops can make decisions across ticks without inventing a mini interpreter or variable system.

Examples:

- run a subcall
- inspect `last_subcall` next tick
- rewrite transcript
- inspect `last_transcript_rewrite` next tick
- capture PTY
- inspect `last_pty_capture`

This is one of the key architectural choices of the runtime:

- richer state snapshots
- bounded runtime-native effects
- no loop DSL

## Advisories

Two important advisory signals are tracked explicitly:

- `context_pressure`
- `doom_loop`

These are runtime-backed observations, not transcript hacks.

The runtime computes them; the loop decides what to do with them.

Example with [`SimpleLoop`](../../../crates/agent-loops/src/simple.rs):

- if pressure says `should_compact`, return `CompactContext`
- if doom-loop repetition is unhandled, queue steering

## A Useful Reading Pattern

When debugging runtime behavior, inspect state in this order:

1. `phase` and `boundary`
2. `pending_runtime_actions`
3. `pending_direct_messages`, `pending_child_reports`, `pending_promoted_pty_events`
4. `pending_tool_calls`, `pending_approval`, `active_wait`
5. `children` or `ptys`
6. advisories like `context_pressure` and `doom_loop`

That sequence usually tells you why the runtime did what it did.

## Related Reading

- [control-flow.md](control-flow.md)
- [decisions-and-events.md](decisions-and-events.md)
- [subagents.md](subagents.md)
- [pty.md](pty.md)
