# Agent Loops and Agent Engines

## Purpose

This document aggregates recent source-level research on competing agent runtimes to help answer a narrower design question for `brain`:

- what is an "agent" at the core runtime level
- what responsibilities belong in a loop or strategy
- what responsibilities belong in a lower-level engine/session runtime
- which features are common enough that our abstractions should anticipate them now

This note is intentionally more architectural than product-focused. It complements the higher-level competitor summaries under [`docs/research/competitors`](./README.md) by tying those summaries back to concrete repocache evidence.

## Table of Contents

- [Purpose](#purpose)
- [Why This Matters for `brain`](#why-this-matters-for-brain)
- [Glossary](#glossary)
  - [Agent](#agent)
  - [Agent Engine](#agent-engine)
  - [Provider](#provider)
  - [Session](#session)
  - [Thread](#thread)
  - [Turn](#turn)
  - [Turn Step](#turn-step)
  - [Tool Phase](#tool-phase)
  - [Completion Boundary](#completion-boundary)
  - [Steering](#steering)
  - [Interruption](#interruption)
  - [Subagent / Delegation](#subagent--delegation)
  - [Channel](#channel)
  - [Event](#event)
- [Existing `brain` Baseline](#existing-brain-baseline)
- [Competitor Research Map](#competitor-research-map)
- [Cross-Competitor Comparison](#cross-competitor-comparison)
  - [1. Codex](#1-codex)
  - [2. OpenCode](#2-opencode)
  - [3. OpenClaw](#3-openclaw)
  - [4. ZeroClaw](#4-zeroclaw)
  - [5. IronClaw](#5-ironclaw)
- [Feature Matrix](#feature-matrix)
- [What an "Agent" Seems to Be in Practice](#what-an-agent-seems-to-be-in-practice)
- [Terminology Applied to Agent Loops](#terminology-applied-to-agent-loops)
- [Proposed Conceptual Split](#proposed-conceptual-split)
  - [Layer 1: Provider](#layer-1-provider)
  - [Layer 2: Agent Engine](#layer-2-agent-engine)
  - [Layer 3: Loop / Strategy](#layer-3-loop--strategy)
- [Why an Evented Engine Looks Necessary](#why-an-evented-engine-looks-necessary)
- [Implications for `brain`](#implications-for-brain)
  - [What the current `AgentLoop` trait gets right](#what-the-current-agentloop-trait-gets-right)
  - [What it likely gets wrong](#what-it-likely-gets-wrong)
  - [What seems worth preserving](#what-seems-worth-preserving)
  - [What likely needs to be added](#what-likely-needs-to-be-added)
- [One Generic Trait vs Two Layers](#one-generic-trait-vs-two-layers)
  - [Option A: One generic trait with stronger defaults/helpers](#option-a-one-generic-trait-with-stronger-defaultshelpers)
  - [Option B: Explicit two-layer design](#option-b-explicit-two-layer-design)
  - [Current recommendation](#current-recommendation)
- [Design Principles to Carry Forward](#design-principles-to-carry-forward)
- [Open Questions for `brain`](#open-questions-for-brain)
- [Recommended Next Discussion](#recommended-next-discussion)

## Why This Matters for `brain`

Today `brain` exposes a small loop abstraction:

- [`AgentLoop`](../../../crates/brain-types/src/agent_loop.rs)

```rust
pub trait AgentLoop: Send + Sync {
    fn run(
        &self,
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        messages: Vec<Message>,
        config: AgentConfig,
        cancel: CancellationToken,
        session_id: Option<Ulid>,
    ) -> EventStream;
}
```

That trait is elegant, but it also pushes nearly all orchestration responsibilities into each loop implementation. In practice, even "simple" loops end up rebuilding:

- provider streaming assembly
- tool-call parsing and dispatch
- transcript mutation
- event emission
- cancellation handling
- loop limits and retry behavior
- completion semantics

The main question is whether a loop trait alone is the right center of the runtime, or whether an "agent" is really a richer session engine with one or more loop strategies layered on top.

## Glossary

This document uses the following terms consistently.

### Agent

An **agent** is the full runtime behavior that coordinates:

- a model provider
- tools
- a session transcript
- policies and approvals
- optional delegation/subagents
- optional multiple modalities or channels

In other words, an agent is usually broader than just "the model" and broader than just "the loop."

### Agent Engine

The **agent engine** is the runtime layer that manages the mechanics of an agent session:

- transcript mutation
- tool execution
- approvals
- interruptions
- steering
- event emission
- session state

This document argues that the engine is a useful architectural concept distinct from loop strategy.

### Provider

A **provider** is the model-facing inference backend. It is responsible for sending and receiving model I/O, including:

- structured messages/content parts
- tool-calling payloads
- streaming deltas
- multimodal request serialization

It is not, by itself, the agent runtime.

### Session

A **session** is the long-lived conversation/work state for one agent thread.

In the common case, a session includes the full model-visible and runtime-relevant history from initial context through the latest event, such as:

- system/developer instructions
- user messages
- assistant messages
- tool calls
- tool results
- approval outcomes
- compaction summaries
- steering inputs
- subagent outputs

In many systems, a session is what must be reconstituted to continue work correctly.

Important nuance:

- some providers support provider-side storage or response threading, which may reduce how much history the client must resend on each call
- even in those systems, the runtime still usually needs a local session concept for policy, replay, tooling, UI, and orchestration

So for architecture purposes, a session should be understood as the canonical agent-thread state, not merely "the exact bytes resent to the provider on every request."

### Thread

A **thread** is often used interchangeably with session in competitor systems.

When a distinction is useful, this document prefers:

- **session** for the runtime state object
- **thread** for the user-facing conversation/workstream identity

In many implementations, they are effectively the same thing.

### Turn

A **turn** is the unit of work initiated by one external input and ending when the runtime reaches a boundary where it must wait for further external direction.

Examples of external input:

- a user message
- a steering message
- a resume/approval event
- a delegated subtask result that unblocks the parent

For a tool-using agent, one turn often contains:

- one or more model inferences
- zero or more tool-call batches
- zero or more tool-result injections

Practical definition:

- a turn starts when the runtime accepts new external intent
- a turn ends when the runtime yields a stable waiting point for the next external intent

That means a turn is broader than a single inference call.

### Turn Step

A **turn step** is one internal iteration within a turn.

Examples:

- one provider inference
- one tool execution phase
- one completion-confirmation phase
- one compaction or recovery step

If a turn is the outer unit of work, then turn steps are the internal phases that advance it.

This term is useful because many loops are iterative inside a single turn.

### Tool Phase

A **tool phase** is the portion of a turn step where requested tools are executed and their results are prepared for reinjection into the session.

This may involve:

- one tool call
- multiple tool calls
- parallel tool calls
- approvals before execution

### Completion Boundary

A **completion boundary** is the point where the runtime decides it should stop autonomous progression and wait for external input.

Examples:

- the model produced a final answer and no follow-up tools are required
- a loop-specific `task_complete` or equivalent was confirmed
- the runtime is waiting for approval
- the runtime has been interrupted and paused

This is the boundary that usually ends a turn.

### Steering

**Steering** is an external input that modifies the current in-progress turn without discarding the session.

Typical examples:

- "stop"
- "do this instead"
- "after the current command, also check X"

Steering is not just another transcript message. In stronger runtimes it often has special semantics such as:

- inject at the next safe boundary
- pause until current tool call ends
- modify pending turn behavior without starting a brand-new session

### Interruption

An **interruption** is an external control signal that attempts to stop or pause the current in-progress turn.

Examples:

- cancel the active provider request
- abort after the current tool finishes
- stop immediately if the runtime can do so safely

Interruption is related to steering but distinct:

- steering changes direction
- interruption halts or pauses execution

### Subagent / Delegation

A **subagent** is a child or delegated agent execution spawned from a parent session.

Implementations differ:

- some model subagents as child sessions
- some model them as delegated loop invocations
- some expose them as tools

For this document, "subagent" means a delegated unit of agent work that has its own local reasoning/tool flow, regardless of how it is implemented internally.

### Channel

A **channel** is an external input/output surface connected to the same runtime.

Examples:

- terminal UI
- web UI
- Discord
- Telegram
- voice/audio input
- IDE integration

Channels matter because one session may receive input from multiple surfaces over time.

### Event

An **event** is a structured runtime occurrence emitted by or consumed by the engine.

Examples:

- user input received
- assistant delta emitted
- tool call requested
- tool finished
- approval requested
- approval granted
- turn completed
- subagent completed

This document uses "event" in the runtime sense, not as a synonym for message.

## Existing `brain` Baseline

Relevant local code:

- [`AgentLoop`](../../../crates/brain-types/src/agent_loop.rs)
- [`SimpleLoop`](../../../crates/brain-loops/src/simple.rs)
- [`RobustLoop`](../../../crates/brain-loops/src/robust.rs)
- [`Terminus2Loop`](../../../crates/brain-loops/src/terminus2.rs)
- [`TerminusKiraLoop`](../../../crates/brain-loops/src/terminus_kira.rs)

The important observation is that the "inner loop" in our agent loops is not accidental. It is the natural tool-using agent cycle:

1. ask model
2. stream or collect model output
3. if tools were requested, execute them
4. append tool results
5. ask model again
6. stop only when a real completion boundary is reached

That inner loop is normal. The problem is that too much shared runtime machinery currently lives inside each individual loop implementation.

## Competitor Research Map

This document builds on the higher-level competitor notes:

- [`Codex`](./Codex.md)
- [`OpenCode`](./OpenCode.md)
- [`OpenClaw`](./OpenClaw.md)
- [`IronClaw`](./IronClaw.md)
- [`ZeroClaw`](./ZeroClaw.md)

And the concrete repocache sources for this pass:

- Codex:
  - [`codex-rs/protocol/src/models.rs`](../../../repocache/openai/codex/codex-rs/protocol/src/models.rs)
  - [`codex-rs/core/src/codex.rs`](../../../repocache/openai/codex/codex-rs/core/src/codex.rs)
  - [`codex-rs/core/src/codex_thread.rs`](../../../repocache/openai/codex/codex-rs/core/src/codex_thread.rs)
- OpenCode:
  - [`packages/opencode/src/session/index.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/index.ts)
  - [`packages/opencode/src/tool/task.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/tool/task.ts)
- OpenClaw:
  - [`docs/concepts/agent-loop.md`](../../../repocache/openclaw/openclaw/docs/concepts/agent-loop.md)
  - [`docs/concepts/system-prompt.md`](../../../repocache/openclaw/openclaw/docs/concepts/system-prompt.md)
  - [`src/gateway/agent-prompt.ts`](../../../repocache/openclaw/openclaw/src/gateway/agent-prompt.ts)
- IronClaw:
  - [`src/agents/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/agents/mod.rs)
  - [`src/providers/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/providers/mod.rs)
- ZeroClaw:
  - [`src/agent/loop_.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs)
  - [`src/providers/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/providers/mod.rs)
  - [`src/tools/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/tools/mod.rs)
  - [`src/channels/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/channels/mod.rs)

## Cross-Competitor Comparison

### 1. Codex

Higher-level summary:

- [`docs/research/competitors/Codex.md`](./Codex.md)

Key source evidence:

- [`codex.rs`](../../../repocache/openai/codex/codex-rs/core/src/codex.rs)
- [`codex_thread.rs`](../../../repocache/openai/codex/codex-rs/core/src/codex_thread.rs)
- [`models.rs`](../../../repocache/openai/codex/codex-rs/protocol/src/models.rs)

What matters architecturally:

- Codex is not organized around a small loop trait. It is organized around a session runtime.
- The runtime owns structured content items, approvals, compaction, rollout/history state, realtime conversation, subagents, and turn lifecycle.
- `CodexThread` exposes a bidirectional session conduit:
  - `submit(...)`
  - `steer_input(...)`
  - `next_event(...)`
- The protocol models structured input items and content parts rather than a flat string transcript.

Most important signal for `brain`:

- steering and interruption are not bolted onto the loop as ad hoc cancellation
- they are first-class session operations

This is the strongest evidence that the real center of the architecture is an evented thread/session engine, not just a `run()` call.

### 2. OpenCode

Higher-level summary:

- [`docs/research/competitors/OpenCode.md`](./OpenCode.md)

Key source evidence:

- [`session/index.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/index.ts)
- [`tool/task.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/tool/task.ts)

What matters architecturally:

- sessions are persistent first-class records with parent/child relationships, forking, permissions, summaries, and workspace linkage
- subagents are implemented as child sessions, not just recursive prompt calls
- the `task` tool resumes or creates a child session, selects an agent, and prompts through that session runtime

Most important signal for `brain`:

- "subagent support" is really a session-management feature
- delegation belongs closer to the engine/session layer than to a one-off tool helper

OpenCode is one of the clearest examples that an "agent" in a product runtime is often a long-lived session object with policies, not merely a function around a provider.

### 3. OpenClaw

Higher-level summary:

- [`docs/research/competitors/OpenClaw.md`](./OpenClaw.md)

Key source evidence:

- [`docs/concepts/agent-loop.md`](../../../repocache/openclaw/openclaw/docs/concepts/agent-loop.md)
- [`docs/concepts/system-prompt.md`](../../../repocache/openclaw/openclaw/docs/concepts/system-prompt.md)

What matters architecturally:

- OpenClaw is gateway-first and multi-channel by design
- its documented loop is serialized per session
- queue modes like collect, steer, and follow-up exist at the outer runtime layer
- the gateway bridges events from an embedded inner runtime into lifecycle, assistant, and tool streams

Most important signal for `brain`:

- multiple input channels and deferred steering are runtime concerns, not prompt tricks
- when several user/control-plane inputs can target the same session, an event-channel model is more natural than a single synchronous `run()`

OpenClaw is especially useful for thinking about always-on sessions, external channels, and "authoritative session lane" execution.

### 4. ZeroClaw

Higher-level summary:

- [`docs/research/competitors/ZeroClaw.md`](./ZeroClaw.md)

Key source evidence:

- [`agent/loop_.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs)
- [`providers/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/providers/mod.rs)
- [`tools/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/tools/mod.rs)
- [`channels/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/channels/mod.rs)

What matters architecturally:

- ZeroClaw is the closest Rust implementation to the direction `brain` is exploring
- it has real provider/tool/channel subsystems
- it has a genuine iterative tool loop
- it already treats channels as first-class, not just UI shells

Most important signal for `brain`:

- a shared engine layer below policy-specific loops is viable
- provider capabilities, tool routing, channels, and approvals want to exist below the "agent persona" level

ZeroClaw is the strongest external evidence that our trait-oriented instincts are directionally right, but also a warning that the central loop can still become a catch-all if it is not separated from runtime mechanics.

### 5. IronClaw

Higher-level summary:

- [`docs/research/competitors/IronClaw.md`](./IronClaw.md)

Key source evidence:

- [`agents/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/agents/mod.rs)
- [`providers/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/providers/mod.rs)

What matters architecturally:

- IronClaw explicitly models agent roles, coordination patterns, and shared context
- the provider trait is real and explicit
- the multi-agent orchestration vocabulary is stronger than the loop/runtime wiring

Most important signal for `brain`:

- "agent" can live above the single-turn loop as a coordination/orchestration concept
- roles, collaboration patterns, and shared artifacts are not the same abstraction as provider/tool execution

IronClaw is a useful reminder that a future Brain architecture may need both:

- a low-level session/runtime abstraction
- a higher-level orchestration abstraction

## Feature Matrix

The main recurring features across these systems are:

| Capability | Codex | OpenCode | OpenClaw | ZeroClaw | IronClaw | Implication for `brain` |
| --- | --- | --- | --- | --- | --- | --- |
| Iterative tool loop | Yes | Yes | Yes, via embedded runtime | Yes | Partly | Core loop remains necessary |
| Structured content items | Yes | Yes | Yes | Yes | Yes | Plain `String` messages are too weak long-term |
| Persistent session/thread | Strong | Strong | Strong | Moderate | Weak | Session should be first-class |
| Subagents/delegation | Strong | Strong | Moderate | Moderate | Strong | Delegation is engine-level, not just loop-level |
| Multimodal input | Yes | Yes | Yes | Yes | Yes | Provider/session model should support content parts |
| Multiple input channels | Moderate | Moderate | Strong | Strong | Moderate | Transport/channel may need to be first-class |
| Steering/interruption | Strong | Moderate | Strong | Moderate | Weak | Event-driven session control matters |
| Approval/policy integration | Strong | Strong | Strong | Strong | Strong | Tool execution should sit near policy |
| Compaction as loop/runtime concern | Strong | Strong | Strong | Strong | Weak | Context management belongs below strategy |

## What an "Agent" Seems to Be in Practice

Across these systems, an "agent" is usually **not** just:

- a model provider
- a tool registry
- a transcript
- a loop function

In the stronger architectures, the core agent runtime is closer to:

- a **sessioned event-driven engine**
- that coordinates model turns, tool execution, approvals, context management, interruptions, steering, and optional delegation over time

Then product-specific "modes" or "agents" sit above that:

- planner
- builder
- reviewer
- subagent
- compactor
- title/summary helper

This suggests that "agent" is overloaded and should probably be split conceptually.

## Terminology Applied to Agent Loops

Using the glossary above, most strong tool-using agents can be described as:

1. The runtime receives external input and starts a **turn**.
2. The loop executes one or more **turn steps**.
3. Some turn steps are **provider inference** steps.
4. Some turn steps are **tool phases**.
5. The engine mutates the **session** after each relevant step.
6. The turn ends at a **completion boundary** where the runtime waits for the next external input.

That framing helps avoid ambiguous language like:

- "the loop did another turn" when it really did another internal inference
- "the session ended" when only the current turn completed
- "the tool call was the message" when it was really one event inside a broader turn

For our own discussions, the cleanest default vocabulary is:

- **session** = full thread state
- **turn** = one externally initiated unit of agent work
- **turn step** = one internal iteration within a turn
- **event** = one emitted or consumed runtime occurrence

## Proposed Conceptual Split

### Layer 1: Provider

The provider is just model I/O capability.

It should handle:

- structured content
- tool-call capable inference
- multimodal message serialization
- streaming output
- capability metadata

It should not own:

- session policy
- tool lifecycle orchestration
- steering queues
- approvals
- subagent management

### Layer 2: Agent Engine

This is the missing center in our current architecture.

It should own:

- session state and transcript mutation
- structured input/output items
- tool execution lifecycle
- approvals and policy hooks
- interruption and steering queues
- context trimming / compaction
- maybe delegation/subagent lifecycle
- event emission

This layer is where the runtime becomes a state machine rather than just a call stack.

### Layer 3: Loop / Strategy

This is the policy layer that decides what to do next.

It should answer questions like:

- when should the model be queried again
- when is the task complete
- when should tools be executed
- when should steering be injected
- when should context be compacted
- when should a subagent be spawned

This is where `SimpleLoop`, `RobustLoop`, `Terminus2Loop`, and `TerminusKiraLoop` fit best.

## Why an Evented Engine Looks Necessary

The steering/interruption examples are the strongest evidence.

Examples from the research:

- Codex exposes `steer_input(...)` on the thread object, not as an afterthought in the loop:
  - [`codex_thread.rs`](../../../repocache/openai/codex/codex-rs/core/src/codex_thread.rs)
- OpenClaw explicitly documents serialized per-session runs and queue modes such as collect, steer, and follow-up:
  - [`docs/concepts/agent-loop.md`](../../../repocache/openclaw/openclaw/docs/concepts/agent-loop.md)
- OpenCode implements subagents as separate sessions launched via the `task` tool:
  - [`tool/task.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/tool/task.ts)

The key insight is that a running session often has multiple concurrent or deferred inputs:

- initial user message
- interrupt signal
- steering message
- approval response
- tool completion
- subagent completion
- external channel input

That maps much more naturally to an evented engine than to a plain synchronous `run()` abstraction.

## Implications for `brain`

### What the current `AgentLoop` trait gets right

- It keeps strategy separate from provider/tool/store at a high level.
- It keeps event streaming visible in the API.
- It allows very different loop styles.

### What it likely gets wrong

- It is too low-level to be the only runtime abstraction.
- It forces each loop to rebuild too much orchestration.
- It has no natural place for:
  - steering queues
  - safe-boundary injection
  - approval pause/resume
  - external channel input mid-turn
  - subagent completion events

### What seems worth preserving

- keep the current loop concept
- keep provider and tool as explicit traits
- keep streamed events as a first-class output

### What likely needs to be added

- a richer session/engine abstraction beneath loops
- structured event inputs, not just an initial message vector
- explicit "safe boundary" semantics for steering and interruption
- cleaner separation between runtime mechanics and loop policy

## One Generic Trait vs Two Layers

There are two plausible directions.

### Option A: One generic trait with stronger defaults/helpers

This would keep one top-level loop trait but provide a much richer runtime context:

- provider turn assembly
- transcript manager
- tool executor
- approval manager
- steering queue
- event sink
- subagent API

Pros:

- smaller public surface
- less visible architectural churn

Cons:

- the trait may stay deceptively simple while the real engine becomes implicit
- loop implementations may still accumulate infrastructure responsibilities

### Option B: Explicit two-layer design

- low-level session/engine runtime
- high-level loop/strategy trait

Pros:

- matches what the strongest competitors are actually doing
- makes steering/interruption/delegation easier to model cleanly
- reduces duplicated orchestration code in loops

Cons:

- more explicit architecture to design up front
- more moving parts

### Current recommendation

Based on this research, the two-layer model is the cleaner long-term direction.

If we keep one top-level trait, it should still be understood as sitting on top of a richer engine/runtime layer. In other words, even the "one trait" approach should probably be implemented internally as a two-layer system.

## Design Principles to Carry Forward

1. **Session first, not prompt first**
   - durable state and boundaries matter more than a single prompt builder

2. **Structured content first**
   - multimodal and tool-rich systems outgrow flat string messages quickly

3. **Evented control flow**
   - interruptions, steering, approvals, and subagent completion are runtime events

4. **Loop as strategy, not plumbing**
   - loop implementations should encode policy, not rebuild the engine

5. **Delegation is not just a tool**
   - subagents often become child sessions, not just nested function calls

6. **Policy should sit close to tool execution**
   - approval and sandbox logic need to be in the execution path, not only in the UI

7. **Channels may become first-class**
   - the stronger multi-surface systems treat channels as more than a transport skin

## Open Questions for `brain`

These questions should be settled before locking in the next major loop/runtime trait boundary:

- Is `AgentLoop` meant to be per-turn, per-session, or both?
- Should steering be modeled as:
  - an injected message
  - a control event
  - or a queued action at the engine layer?
- Do we want subagents to be:
  - tools
  - child sessions
  - or a higher-level orchestration system?
- Is `Transport` enough for multi-channel input, or do we need a richer channel abstraction?
- Should compaction be loop-owned, engine-owned, or shared?
- Should event streams be bidirectional at the runtime boundary?

## Recommended Next Discussion

The next useful design exercise is not "rewrite `AgentLoop` immediately." It is to define the minimum internal vocabulary for a sessioned agent engine:

- session
- event
- action
- boundary
- steering
- interruption
- delegation
- channel
- approval

Once those concepts are stable, the trait shape will likely become much clearer.
