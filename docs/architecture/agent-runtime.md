# Agent Runtime

`agent-runtime` is the reusable storeless session engine that now owns the
mechanics of provider execution, tool execution, transcript mutation,
safe-boundary control flow, child runtimes, and cross-agent messaging.

It is intentionally narrower than a full product runtime and intentionally
broader than a loop strategy. The point is to keep hard execution mechanics in
one place while keeping orchestration policy swappable.

## Design Rule

The core boundary is:

- `agent-runtime` owns **mechanisms and invariants**
- `agent-loops` owns **policy decisions**
- prompts or higher-level strategies own **semantic recovery behavior**

This rule matters because the runtime is now sophisticated enough that it could
easily become an opinionated agent brain by accident. It should not.

## Runtime Responsibilities

`agent-runtime` should own:

- transcript state and safe transcript mutation
- provider streaming, retries, and output normalization
- tool execution lifecycle and tool-result reinjection
- approval, interruption, pause/resume, and wait mechanics
- child runtime lifecycle and registry routing
- mailbox/direct-message storage and safe-boundary injection
- runtime-side detection helpers such as context pressure and repeated-tool
  signatures
- optional hard safety rails that are explicitly configured

These behaviors are hard to get right if every loop has to reimplement them.

## Loop Responsibilities

Loop strategies should own:

- when to compact context
- when to steer after a doom-loop warning
- whether to ignore or escalate advisory warnings
- whether to wait, continue, or delegate
- whether to use background or blocking child behavior
- how to sequence custom subcalls, transcript rewrites, and transcript appends
  across multiple ticks

Loops should request runtime actions such as `RunProvider`, `ExecuteToolBatch`,
`QueueSteering`, `WaitForAgents`, `CompactContext`, `RunSubcall`,
`RewriteTranscript`, or `AppendTranscriptMessages`, but should not manually
mutate transcript storage or consume provider streams.

## Action Path Rule

There is one canonical execution path for **state-changing runtime actions**:

1. an external caller or native runtime tool requests an action
2. the runtime records that action as a pending required runtime action
3. the engine drains required pending actions before ordinary loop policy
4. the runtime executes the corresponding `LoopDecision`

This means externally requested actions are not executed "directly" in a
separate control plane anymore. They still flow through the same runtime action
path as loop-chosen work.

The important distinction is now:

- **direct reads**
  - immediate query-style operations that do not change runtime state
  - examples: `list_agents`, `read_agent_mail`, `list_ptys`, `get_pty`,
    `read_pty_events`
- **required runtime actions**
  - state-changing operations that must be sequenced through the canonical
    runtime action path
  - examples: spawn/message/wait/interrupt child runtimes, transcript rewrites,
    subcalls, and mutating PTY actions

Loops do not get discretionary veto over required runtime actions. Their role
is to preserve one understandable execution order, not to silently ignore
explicit requested work.

## Bounded Declarative Effect Rule

The loop surface should stay declarative, but it should not stay *tiny* forever.
The right evolution is:

- add more **runtime-native effects**
- expose more **typed runtime state and operation results**
- let loops branch in Rust across ticks

The wrong evolution is to turn `LoopDecision` into a mini workflow language.

Explicit non-goals for the loop surface:

- no `If`
- no `While`
- no `ForEach`
- no plan-level variables or registers
- no user-defined transform DSL embedded in the decision type

Advanced loops should still be expressible, but they should work by inspecting
updated runtime state on the next tick, not by returning nested control flow.

## Advanced Loop Surface

The newer runtime path should support two layers of loop usage without becoming
two different architectures:

- **simple loops** that only use high-level runtime actions such as provider
  steps, tool batches, waits, compaction, and steering
- **advanced loops** that also use isolated provider subcalls and concrete
  transcript edits to implement custom handoff or recovery behavior

This is why the runtime now exposes:

- `RunSubcall`
- `RewriteTranscript`
- `AppendTranscriptMessages`
- typed `last_subcall`, `last_transcript_rewrite`, `last_transcript_append`,
  and bounded `recent_operations` state

Simple loops are just the small-subset users of the same runtime surface.

## Prompt And Strategy Responsibilities

Prompting and higher-level strategy layers should own:

- the wording of recovery prompts
- domain-specific notions of being stuck
- when to re-plan versus keep going
- whether to delegate to another agent versus try another tool

This is the layer where product personality belongs.

## Compaction Rule

Transcript compaction is split deliberately:

- runtime computes **context pressure**
- loops decide whether to request `CompactContext`
- runtime performs the summary subcall and rewrites transcript state safely

That keeps compaction available as a reusable primitive without forcing one
global compaction policy on every loop.

For more custom flows, loops can also use:

- `RunSubcall` to produce a handoff or summary
- `RewriteTranscript` to replace the transcript with a loop-authored handoff
  shape

That is how Terminus-style compaction should eventually fit into the new
runtime path without hard-coding every compaction style into one built-in
runtime prompt.

## Doom-Loop Rule

Repeated-tool-cycle handling is also split deliberately:

- runtime tracks repeated tool-call signatures and exposes doom-loop warnings
- loops decide how to react to advisory warnings
- runtime may still enforce an explicit hard-stop threshold when configured

This preserves flexibility for loops that want custom recovery behavior while
still allowing the runtime to provide hard safety rails when needed.

## What This Prevents

This boundary is meant to prevent two failure modes:

- loops becoming giant monoliths that reimplement retries, transcript
  bookkeeping, and control flow
- the runtime silently absorbing all orchestration policy and making custom
  loops meaningless

The right shape is:

- simple loop implementation
- robust runtime execution
- policy still replaceable

## PTY Runtime Capability

Persistent PTY or terminal sessions are now treated as runtime-managed
resources rather than generic tool calls.

The PTY direction follows the same lesson as child runtimes:

- PTYs have identity
- PTYs have lifecycle
- PTYs have mutable state over time
- PTYs emit events that may be observed or promoted safely

The runtime-native PTY surface now centers on:

- stable `PtyId` values
- lifecycle and query actions such as open, list, get, resize, interrupt,
  background, and close
- execution actions such as raw input writes and command batches
- structured snapshots and execution results
- explicit event subscriptions
- optional safe-boundary promotion of subscribed PTY events into `developer`
  transcript messages

This keeps the boundary consistent with the rest of the runtime:

- runtime owns PTY state, buffering, event emission, and safety
- loops choose when to use PTY effects
- PTY events are runtime facts first and transcript content only when
  explicitly subscribed for promotion

The important non-goal remains the same: loops should request runtime-native
PTY operations and inspect structured PTY state, not script an imperative
terminal interpreter through the loop decision surface.
