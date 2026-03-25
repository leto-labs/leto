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

- Local `brain` / `agent-runtime` references:
  - [`SimpleLoop`](../../../crates/agent-loops/src/simple.rs)
  - [`Terminus2Loop`](../../../crates/brain-loops/src/terminus2.rs)
  - [`agent-runtime` command surface](../../../crates/agent-runtime/src/command.rs)
  - [`agent-runtime` loop decision surface](../../../crates/agent-runtime/src/loop_strategy.rs)
  - [`agent-runtime` session state](../../../crates/agent-runtime/src/session.rs)
  - [`agent-runtime` engine/tool bridge](../../../crates/agent-runtime/src/engine.rs)
- Codex:
  - [`codex-rs/core/src/agent/control.rs`](../../../repocache/openai/codex/codex-rs/core/src/agent/control.rs)
  - [`codex-rs/core/src/session_prefix.rs`](../../../repocache/openai/codex/codex-rs/core/src/session_prefix.rs)
  - [`codex-rs/app-server-protocol/src/protocol/thread_history.rs`](../../../repocache/openai/codex/codex-rs/app-server-protocol/src/protocol/thread_history.rs)
- OpenCode:
  - [`packages/opencode/src/session/index.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/index.ts)
  - [`packages/opencode/src/session/prompt.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts)
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
  - [`src/tools/delegate.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/tools/delegate.rs)
  - [`docs/reference/api/config-reference.md`](../../../repocache/zeroclaw-labs/zeroclaw/docs/reference/api/config-reference.md)
  - [`src/channels/mod.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/channels/mod.rs)
- Gastown:
  - [`AGENTS.md`](../../../repocache/steveyegge/gastown/AGENTS.md)
  - [`internal/cmd/nudge.go`](../../../repocache/steveyegge/gastown/internal/cmd/nudge.go)

## Cross-Competitor Comparison

Before comparing named systems, it helps to separate three buckets that often get conflated:

| Bucket | Representative systems | What is really being compared |
| --- | --- | --- |
| Single-agent shell harness | `Terminus2Loop`, `TerminusKiraLoop` | One agent loop repeatedly driving a shell or terminal tool. |
| Native subagent runtime | Codex, OpenCode, ZeroClaw delegate mode, current `agent-runtime` | A runtime/session substrate that can create or control child units of work with their own model/tool flow. |
| Orchestration fabric above runtimes | Gastown, parts of IronClaw | Messaging, wake-up, queueing, routing, and coordination above any one session runtime. |

That split is important for `brain` because "multi-agent" can mean at least three different things:

- a single agent that is very good at driving a terminal
- a session runtime that can spawn and control child runtimes
- an external coordination plane that routes work and messages between otherwise separate sessions

### 1. Terminus-style single-agent shell control

Key local source evidence:

- [`Terminus2Loop`](../../../crates/brain-loops/src/terminus2.rs)
- [`TerminusKiraLoop`](../../../crates/brain-loops/src/terminus_kira.rs)

What matters architecturally:

- Terminus is still one agent loop, even when it looks operationally sophisticated.
- The autonomy comes from a repeated provider -> parse -> `terminal_session` / shell -> observe cycle.
- The runtime surface is centered on keystrokes, terminal snapshots, shell execution, and explicit `task_complete` confirmation.
- There is no first-class child runtime identity, no parent/child registry, no mailbox, and no background delegated worker lifecycle.

Most important signal for `brain`:

- a tmux/terminal harness can feel very agentic without having any native subagent substrate
- if `brain` only grows stronger terminal tooling, it will still be qualitatively different from Codex/OpenCode-style subagents
- this is the right baseline for comparison with "one smart terminal worker", not with a real multi-runtime control plane

### PTY management comparison

Looking specifically at terminal/PTY handling across the stronger references:

| System | PTY shape | Strongest lesson |
| --- | --- | --- |
| Terminus2 / Harbor | one persistent terminal session used as the core execution substrate | terminal state is long-lived and must be observable across many turns |
| Terminus Kira | persistent terminal plus stricter completion, timeout, and interrupt behavior | richer PTY execution semantics matter more than a loop DSL |
| OpenCode | PTY as a first-class managed subsystem with ids, lifecycle, write/resize/connect/list behavior | PTY should be treated as a managed runtime resource, not a generic tool |
| Codex | robust PTY/process-group execution substrate with buffering and cleanup semantics | low-level interrupt, stream, and lifecycle behavior must be reliable |
| Gastown | tmux/session environment used as durable orchestration substrate | long-lived terminal sessions need explicit lifecycle and operational hygiene |

The practical conclusion for `brain` is:

- a one-shot shell tool is not enough for Terminus-style behavior
- a generic `terminal_session` tool is directionally right but still too tool-shaped
- runtime-native PTY support should combine:
  - OpenCode-style identity/lifecycle
  - Codex-style execution/interrupt semantics
  - Gastown-style long-lived session hygiene

### 2. Codex

Higher-level summary:

- [`docs/research/competitors/Codex.md`](./Codex.md)

Key source evidence:

- [`agent/control.rs`](../../../repocache/openai/codex/codex-rs/core/src/agent/control.rs)
- [`session_prefix.rs`](../../../repocache/openai/codex/codex-rs/core/src/session_prefix.rs)
- [`thread_history.rs`](../../../repocache/openai/codex/codex-rs/app-server-protocol/src/protocol/thread_history.rs)

What matters architecturally:

- Codex is not organized around a small loop trait. It is organized around a session/thread runtime.
- `AgentControl` is explicitly a control-plane handle for multi-agent operations shared across a user session.
- Child runtimes are real thread/session objects with stable ids and explicit operations:
  - `spawn_agent(...)`
  - `resume_agent_from_rollout(...)`
  - `send_input(...)`
  - `interrupt_agent(...)`
  - `shutdown_agent(...)`
  - `subscribe_status(...)`
- Parent sessions get model-visible subagent notifications injected when watchers observe child completion.
- Parent model context can also carry a rendered `<subagents>` block listing current child agents.
- The protocol/history layer knows about collab spawn, interaction, wait, close, and resume operations as first-class history items.

Most important signal for `brain`:

- steering, interruption, waiting, and redirection are not bolted onto the loop as ad hoc cancellation
- Codex already treats "child agent lifecycle" as session/runtime state, not merely as tool output
- this is the strongest reference for native subagents as a registry + control API + model-visible status surface

### 3. OpenCode

Higher-level summary:

- [`docs/research/competitors/OpenCode.md`](./OpenCode.md)

Key source evidence:

- [`session/index.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/index.ts)
- [`session/prompt.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts)
- [`tool/task.ts`](../../../repocache/anomalyco/opencode/packages/opencode/src/tool/task.ts)

What matters architecturally:

- Sessions are persistent first-class records with parent/child relationships, forking, permissions, summaries, and workspace linkage.
- Subagents are implemented as child sessions, not just recursive prompt calls.
- The `task` tool resumes or creates a child session, selects an agent, and prompts through that session runtime.
- If the chosen subagent definition does not specify a model, OpenCode falls back to the spawning assistant message's `providerID` and `modelID`.
- The default model-facing surface is still mostly "spawn or resume a child session, run the delegated task, return `<task_result>`."

Most important signal for `brain`:

- "subagent support" is really a session-management feature
- config/model inheritance defaults matter in practice and should be deterministic
- OpenCode is native enough to have child sessions, but the default UX is still more final-result-oriented than mailbox-oriented

### 4. OpenClaw

Higher-level summary:

- [`docs/research/competitors/OpenClaw.md`](./OpenClaw.md)

Key source evidence:

- [`docs/concepts/agent-loop.md`](../../../repocache/openclaw/openclaw/docs/concepts/agent-loop.md)
- [`docs/concepts/system-prompt.md`](../../../repocache/openclaw/openclaw/docs/concepts/system-prompt.md)

What matters architecturally:

- OpenClaw is gateway-first and multi-channel by design.
- Its documented loop is serialized per session.
- Queue modes like collect, steer, and follow-up exist at the outer runtime layer.
- The gateway bridges events from an embedded inner runtime into lifecycle, assistant, and tool streams.

Most important signal for `brain`:

- multiple input channels and deferred steering are runtime concerns, not prompt tricks
- when several user/control-plane inputs can target the same session, an event-channel model is more natural than a single synchronous `run()`

### 5. ZeroClaw

Higher-level summary:

- [`docs/research/competitors/ZeroClaw.md`](./ZeroClaw.md)

Key source evidence:

- [`agent/loop_.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs)
- [`tools/delegate.rs`](../../../repocache/zeroclaw-labs/zeroclaw/src/tools/delegate.rs)
- [`config-reference.md`](../../../repocache/zeroclaw-labs/zeroclaw/docs/reference/api/config-reference.md)

What matters architecturally:

- ZeroClaw is the closest Rust implementation to the direction `brain` is exploring on providers/tools/channels.
- It has a real iterative tool loop and treats channels as first-class.
- Delegation exists as a `delegate` tool backed by named sub-agent configs.
- Those sub-agents can be single-prompt by default or "agentic" with their own filtered tool-call loop and recursion-depth limits.
- The current implementation still reads more like delegated tool execution than a live child-runtime registry with durable bidirectional messaging.

Most important signal for `brain`:

- a shared engine layer below policy-specific loops is viable
- delegate-tool ergonomics are useful, but a delegate tool alone is not the same as a first-class child-runtime graph
- ZeroClaw is a good comparison point for "sub-agent profiles + delegate tool", not for a registry with direct-vs-mail messaging

### 6. IronClaw

Higher-level summary:

- [`docs/research/competitors/IronClaw.md`](./IronClaw.md)

Key source evidence:

- [`agents/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/agents/mod.rs)
- [`providers/mod.rs`](../../../repocache/JoasASantos/ironclaw/src/providers/mod.rs)

What matters architecturally:

- IronClaw explicitly models agent roles, coordination patterns, and shared context.
- The provider trait is real and explicit.
- The collaboration vocabulary is stronger on roles/patterns/shared artifacts than on per-child runtime lifecycle controls.
- Shared context carries inter-agent messages and artifacts across sequential, parallel, debate, hierarchical, and pipeline patterns.

Most important signal for `brain`:

- "agent" can live above the single-turn loop as a coordination/orchestration concept
- roles, collaboration patterns, and shared artifacts are not the same abstraction as provider/tool execution
- IronClaw is closer to a built-in orchestration framework than to a fine-grained child-runtime control plane

### 7. Gastown

Key source evidence:

- [`AGENTS.md`](../../../repocache/steveyegge/gastown/AGENTS.md)
- [`internal/cmd/nudge.go`](../../../repocache/steveyegge/gastown/internal/cmd/nudge.go)

What matters architecturally:

- Gastown is not primarily a single runtime with native child sessions. It is a multi-agent environment and coordination fabric.
- It separates immediate session delivery (`gt nudge`) from persistent inbox delivery (`gt mail`).
- `gt nudge` has several delivery modes, including queueing and wait-for-idle delivery before injection into the target session.
- `gt mail` is durable, queryable, and restart-safe. Agents explicitly read inbox state.
- The system is comfortable with wake-up, routing, durability, addressing, and daemon/session management as first-class concerns.

Most important signal for `brain`:

- Gastown is the strongest local reference for the distinction between direct notification and pull-based mail
- it validates the "DM vs mailbox" split we are adding to `agent-runtime`
- it is best understood as orchestration infra above the runtime, not as a substitute for runtime-native child lifecycle

### 8. Current `agent-runtime`

Key local source evidence:

- [`command.rs`](../../../crates/agent-runtime/src/command.rs)
- [`loop_strategy.rs`](../../../crates/agent-runtime/src/loop_strategy.rs)
- [`session.rs`](../../../crates/agent-runtime/src/session.rs)
- [`engine.rs`](../../../crates/agent-runtime/src/engine.rs)
- [`SimpleLoop`](../../../crates/agent-loops/src/simple.rs)

What matters architecturally:

- The current design has moved decisively away from "subagent as an ad hoc tool helper" and toward a runtime-native registry/control model.
- Parent and child runtimes have stable ids, typed spawn/wait/message operations, child snapshots, explicit wait policy, and safe-boundary delivery rules.
- `spawn_agent`, `message_agent`, `read_agent_mail`, `interrupt_agent`, `list_agents`, and `wait_agent` are model-visible native tools, but they are facades over runtime actions.
- Cross-agent messaging distinguishes:
  - direct messages surfaced at the next safe boundary
  - mail that stays in the mailbox until pulled with filters
- Child reports and direct messages are promoted into model-visible transcript messages at safe boundaries.
- Today those injected wrappers use `MessageRole::Developer`, which is a reasonable choice when the content is runtime-authored control-plane context rather than literal user intent.

Most important signal for `brain`:

- our current runtime is no longer in the "single loop with nicer tools" bucket
- it is now much closer to Codex/OpenCode on the core direction
- the biggest remaining gap relative to systems like Gastown is broader orchestration fabric and routing policy above the runtime

## Feature Matrix

There are really two different matrices worth tracking:

1. a broad engine/runtime matrix
2. a detailed subagent capability matrix

### Engine / Runtime Matrix

| Capability | Terminus-style shell loop | Codex | OpenCode | OpenClaw | ZeroClaw | IronClaw | Gastown | Current `agent-runtime` | Implication for `brain` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Iterative tool loop | Yes | Yes | Yes | Yes, via embedded runtime | Yes | Partly | No, not the core abstraction | Yes | Core loop remains necessary |
| Persistent session/thread | Weak | Strong | Strong | Strong | Moderate | Weak | Strong, but at orchestration level | Strong | Session should be first-class |
| Native child runtime identity | No | Strong | Strong | Moderate | Moderate | Weak | No, external sessions | Strong | Child runtimes should not just be recursive helper calls |
| Steering/interruption | Weak | Strong | Moderate | Strong | Moderate | Weak | Strong, via external commands | Strong | Evented session control matters |
| Multiple input/channels | No | Moderate | Moderate | Strong | Strong | Moderate | Strong | Moderate | Channel/transport concerns do not belong only in prompts |
| Approval/policy integration | Weak | Strong | Strong | Strong | Strong | Strong | Moderate | Strong | Tool execution should sit near policy |
| Compaction/context management | Loop-local | Strong | Strong | Strong | Strong | Weak | External/session recovery | Strong | Context management belongs below strategy |

### Detailed Subagent Capability Matrix

This is the matrix that matters most for the current `agent-runtime` discussion. It focuses on the features that most clearly distinguish:

- single-agent shell loops
- native child-runtime systems
- orchestration fabrics

| System | Spawn multiple subagents | Background children | Parent -> child re-input / steer | Cross-agent direct messaging | Pull mailbox / inbox read | Wait semantics | Can wait, then release hold and keep child running | Child reports / model-visible status | Discovery / listing | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Terminus-style shell loop | No | No | n/a | No | No | No child wait concept | n/a | terminal state only | n/a | Strong single-worker terminal autonomy, not multi-agent runtime |
| Codex | Yes | Yes | Yes via `send_input`, plus interrupt/resume/shutdown | Partial; strong control-plane interaction, less explicit mail model | Weak compared to a real mail system | Strong explicit collab wait/history items | Likely yes in practice because control and waiting are separate collab operations, but less visibly productized than in Gastown-style mail | Strong: subagent notifications and environment-context listing | Strong | Best reference for native runtime subagents with control-plane APIs |
| OpenCode | Yes via child sessions | Moderate | Limited; mostly resume or re-prompt child session | Weak | No general mailbox surface | Mostly tied to task execution / resume flow | Weak-to-moderate | Final result strong, incremental child messaging weaker | Strong session graph | Native child sessions, but model UX remains task-tool-centric |
| ZeroClaw | Moderate via named delegate configs | Weak-to-moderate | Limited; delegate again rather than fully control a live child | Weak | No | Mostly tool-call wait semantics | Weak | Returned delegate output, not rich child lifecycle | Configured agent names | Better delegate tool than runtime-native child graph |
| IronClaw | Yes at orchestration-pattern level | Moderate | Pattern-dependent | Shared-context messages rather than explicit DM/mail split | Shared context, not inbox/mail | Pattern-dependent | Pattern-dependent | Shared artifacts/messages, not a specific child-report surface | Strong at role/pattern level | Better at orchestration patterns than child-runtime control |
| Gastown | Yes across sessions | Yes | Yes via external commands like `gt nudge` | Strong | Strong | External wake/queue semantics, not a child wait API inside one runtime | Yes: queued or timed nudges can effectively release immediate hold while work continues | External to transcript unless agent reads/responds | Strong via addresses and inboxes | Best reference for orchestration-level direct vs mail split |
| Current `agent-runtime` | Yes | Yes by default (`Background`) | Yes via `SendAgentInput` / `message_agent` and interrupt/pause/resume actions | Strong: `Direct` messages routed by registry id | Strong: `Mail` plus `read_agent_mail` filters | Strong `WaitRequest` with timeout policy | Strong: default timeout releases hold and leaves child running; optional interrupt on timeout | Strong: child reports and direct messages injected at safe boundaries | Conservative by default (`list_agents` -> direct children), broader routing by known id | This is now a real child-runtime control plane, not just a nicer tool loop |

### Specific capability observations

#### Ability to spawn multiple subagents

- Codex, OpenCode, IronClaw, Gastown, and the current `agent-runtime` all clearly support multiple active child/peer units rather than a single `active_child` slot.
- Terminus-style loops do not; they remain one active agent driving tools.
- ZeroClaw supports multiple configured delegate profiles, but its core surface still behaves more like repeated delegated calls than a durable set of live child runtimes.

#### Backgrounding

- Terminus has no real background child concept.
- OpenCode can leave child sessions around, but its primary UX is still "run delegated task and return result."
- Gastown is strong here because sessions and mail live outside any one synchronous turn.
- The current `agent-runtime` is explicitly designed for background children as the default spawn posture, which is a major architectural shift from single-loop delegation.

#### Cross-agent communication

- Gastown is the clearest reference for separating immediate notify from durable inbox.
- Codex has strong parent/child control-plane communication and model-visible notifications, but less of an explicit durable mailbox abstraction.
- OpenCode is still more result-oriented than message-oriented.
- The current `agent-runtime` now sits in an interesting middle position:
  - more native runtime control than Gastown
  - more explicit direct-vs-mail messaging than Codex/OpenCode
  - still missing some of the broader orchestration ergonomics of Gastown

#### Waiting, then backgrounding later

- This is one of the most important differentiators because many systems blur "spawn" and "block."
- In Terminus there is nothing comparable because there is no child runtime.
- In many delegate-tool designs, waiting is implicit in the tool call, so "release hold and let it continue" is awkward or unsupported.
- Gastown achieves a similar effect externally through queueing, nudging, and mail.
- The current `agent-runtime` now treats this explicitly:
  - spawn defaults to background
  - waiting is a separate policy object
  - timeout can release the hold while leaving the child alive
  - timeout can optionally interrupt instead

This is one of the strongest arguments that the current design has moved beyond a tool helper into a real runtime substrate.

### Full Scorecard

The capability matrix above says what each system can do. This scorecard is a more opinionated evaluation of how useful each system is for running a real team of agents cleanly.

Scoring dimensions:

- **Power**: how much multi-agent behavior the system can express
- **Flexibility**: how easily it supports different team shapes without redesign
- **Implementation**: how concrete/coherent the feature looks in source, not just in marketing or docs
- **Stability**: how well the architecture protects against chaos via explicit lifecycle, routing, waiting, or delivery semantics
- **Team fit**: how suitable it is for a true long-running team of agents rather than a single high-agency worker

| System | Power | Flexibility | Implementation | Stability | Team fit | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Terminus-style shell loop | `C` | `D+` | `B` | `C+` | `D` | Strong single-worker autonomy; weak as team infrastructure because there is no child-runtime substrate. |
| Codex | `A` | `A-` | `A-` | `A-` | `A` | Best native child-runtime reference in this set. Real control-plane operations, status tracking, and model-visible lifecycle. |
| OpenCode | `B+` | `B+` | `B+` | `B` | `B+` | Strong child-session model, but the default UX still leans toward task/result more than active multi-agent coordination. |
| OpenClaw | `B` | `B+` | `B` | `B+` | `B` | Strong runtime/channel thinking, but less clearly centered on true subagent team control. |
| ZeroClaw | `B` | `B` | `B` | `B-` | `B-` | Useful delegate-tool model, but weaker on durable child-runtime control and communication. |
| IronClaw | `B+` | `A-` | `B-` | `B` | `B+` | Strong orchestration vocabulary and role patterns; weaker on precise child-runtime lifecycle semantics. |
| Gastown | `A-` | `A` | `B+` | `A-` | `A` | Best orchestration/message fabric in the set. Especially strong on wake-up, queueing, durable mail, and operational cleanliness. |
| Current `agent-runtime` | `A-` | `A-` | `B+` | `A-` | `A-` | Architecturally strong: typed runtime actions, background children, direct vs mail, explicit wait/release semantics. Lower maturity than Codex, but the design direction is very good. |

### Codex vs Gastown Scorecard

These are the two most important reference points because they represent two different kinds of strength.

| Dimension | Codex | Gastown | Which is stronger |
| --- | --- | --- | --- |
| Native child-runtime control | Excellent | Weak | Codex |
| Spawn/resume/interrupt/wait semantics | Excellent | Moderate, but externalized | Codex |
| Direct addressing and wake-up | Good | Excellent | Gastown |
| Durable mailbox / pull model | Moderate | Excellent | Gastown |
| Safety against message loss | Good | Excellent | Gastown |
| Model-visible subagent state | Excellent | Weak-to-moderate | Codex |
| Team-wide operational cleanliness | Good | Excellent | Gastown |
| Fit for embedding in one runtime | Excellent | Weak | Codex |
| Fit for coordinating many sessions/processes | Moderate | Excellent | Gastown |

### What This Means for `brain`

If the goal is a true team of agents that stays clean and stable under load, the right move is not to choose Codex *or* Gastown. It is to borrow the right layer from each.

What to take from Codex:

- stable child runtime identity
- native control-plane actions for spawn, redirect, interrupt, pause/resume, status, and wait
- child lifecycle as first-class runtime state
- safe model-visible child status and child-result reporting
- subagent discovery/listing in runtime context

What to take from Gastown:

- explicit distinction between immediate notify and durable mail
- wake-up and wait-idle delivery semantics
- delivery modes that avoid losing messages when agents are busy
- stronger ideas around addressing, queueing, and asynchronous coordination
- operational assumption that not every target is currently idle or even attached

What not to copy blindly from Codex:

- treating mailbox concerns as secondary forever
- assuming the main missing piece is only spawn/control; team systems also need durable routing and pull-based inboxes

What not to copy blindly from Gastown:

- pushing too much coordination outside the runtime too early
- relying on external session/process orchestration before the runtime-level child model is solid

Recommended synthesis for `brain`:

- keep the current native child-runtime model
- keep direct-vs-mail as a first-class split
- strengthen mail durability, addressing, and read/ack semantics in the direction Gastown suggests
- strengthen status subscription, lifecycle observability, and UI/model-visible child state in the direction Codex suggests
- preserve non-destructive wait semantics so "wait now, release later, child keeps running" remains a core primitive rather than an accident

### Future Runtime Improvements

These are intentionally framed as **runtime / SDK / API** improvements, not product CLI features. The comparison in this document is primarily useful insofar as it sharpens what should exist in the core `BrainRuntime` surface and its underlying engine.

#### 1. Add durability and recovery to the runtime substrate

The current `agent-runtime` design is architecturally strong, but it is still storeless and in-memory-first. For a true team runtime, the next maturity step is:

- durable child snapshot state
- durable mailbox state
- durable active wait state
- restart/recovery behavior for background children
- explicit orphan/rejoin handling when the host process restarts

This is the biggest gap between "good runtime model" and "production-grade team runtime."

#### 2. Strengthen runtime-level routing and authorization policy

The registry can already route by runtime id, but a mature team runtime should make policy first-class:

- who may message whom
- who may interrupt whom
- who may discover whom
- whether sibling direct messaging is allowed
- whether some runtimes are control-only, review-only, or read-only
- whether message classes are informational vs control-bearing

This belongs in the runtime/API layer, not in prompt conventions.

#### 3. Deepen mailbox semantics without collapsing them into transcript input

The current direct-vs-mail split is good. The next step is to make the mailbox model more complete:

- explicit ack / mark-read / claim semantics
- message priority
- richer threading/reply linkage
- better pull/query filters
- stronger guarantees around non-lossy delivery and retry behavior

Gastown is a useful reference here, but the goal is a runtime-native mailbox API, not a CLI mail clone.

#### 4. Add richer wait/join semantics to the runtime API

`WaitRequest` is already a strong primitive. The future runtime surface should probably grow beyond simple blocking on a list of ids:

- wait for any
- wait for all
- wait for quorum
- wait until timeout then downgrade to background
- wait groups / join handles
- dependency-aware waiting

This keeps "waiting but then backgrounding" as a first-class runtime behavior rather than an implementation accident.

#### 5. Improve addressability without weakening identity

Stable runtime ids should remain canonical, but the runtime surface will be easier to use if it also supports:

- labels
- roles
- tags
- parent-relative aliases
- filtered agent listing and lookup

This should be additive over canonical ids, not a replacement for them.

#### 6. Add higher-level orchestration helpers above the core primitives

The current runtime has good low-level actions. A future layer can make common team patterns easier without weakening the substrate:

- fan-out / collect helpers
- handoff helpers
- escalation/review helpers
- assignment helpers
- merge/join helpers

These should be built *on top of* the runtime API, not by skipping the runtime API.

#### 7. Improve model-visible child/report representation

The current `MessageRole::Developer` wrappers are defensible, but they are still wrapper-based. A future runtime/API iteration should consider:

- first-class structured child-report content
- first-class structured agent-message content
- clearer provenance fields
- safer parsing for downstream consumers and tools

This is lower priority than durability and policy, but it is part of runtime maturity.

#### 8. Preserve the current architectural direction

The most important recommendation is negative: do **not** regress back toward:

- subagents as only one-shot tool helpers
- recursive parent-owned child objects
- implicit blocking spawn semantics
- transcript-only messaging with no mailbox distinction

The current direction is already much closer to the right runtime model. The work ahead is mostly about hardening and extending it, not replacing it.

### Advanced Loop Fit And Non-Goals

The next architectural question is not "should the runtime grow more hidden
policy?" It is "how expressive can the declarative loop surface become without
turning into an interpreter?"

The strongest conclusion from the legacy loops plus the external references is:

- keep one loop architecture
- keep loops declarative
- let loops branch in Rust across ticks
- add more runtime-native effects and richer loop-visible state
- do **not** add plan-level `If`, `While`, variables, or nested workflow DSLs

#### Reference read-through

- [`Terminus2Loop`](../../../crates/brain-loops/src/terminus2.rs) and Harbor's
  [`terminus_2.py`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/terminus_2.py)
  show that advanced loops need more than threshold-based compaction. They need
  custom handoff summarization, transcript rewrite, parser-repair policy, and a
  persistent terminal substrate.
- [`TerminusKiraLoop`](../../../crates/brain-loops/src/terminus_kira.rs) and
  official
  [`terminus_kira.py`](../../../repocache/krafton-ai/KIRA/terminus_kira/terminus_kira.py)
  push harder on runtime-native terminal and multimodal behavior: command
  batches, timeout/interrupt semantics, completion confirmation, and image-read
  subcalls.
- Codex and OpenCode validate typed session/runtime actions, explicit
  compaction/subtask operations, and clean stateful control planes.
- Gastown validates delivery-mode distinctions and durable asynchronous
  coordination, but it does not imply that the loop return type should become a
  general orchestration DSL.
- Letta Code is a useful contrast because it shows that some "advanced loop"
  pressure is really about persistent agent memory and stateful runtime design,
  not more control-flow power in the loop surface.

#### Advanced-loop support matrix

| Feature | Current runtime | Needs bounded declarative effect expansion | Needs PTY-native runtime support | Out of scope |
| --- | --- | --- | --- | --- |
| Advisory compaction / doom-loop state | Yes | No | No | No |
| Built-in compaction | Yes | No | No | No |
| Multi-step handoff summarization across ticks | Partial | Yes | No | No |
| Concrete transcript rewrite with preserved prefix/tail chosen by the loop | No | Yes | No | No |
| Loop-authored transcript append of runtime-authored messages | No | Yes | No | No |
| Tagged provider subcalls with loop-visible results | No | Yes | No | No |
| Custom parser-repair and completion-confirmation policy | Partial | Yes | No | No |
| Background child orchestration, wait, and release-hold | Yes | No | No | No |
| Persistent PTY session capture and command execution | No | No | Yes | No |
| PTY timeout + interrupt semantics | No | No | Yes | No |
| PTY incremental output / visible-screen observations | No | No | Yes | No |
| Multimodal image-read flow like KIRA | No | Yes | Yes | No |
| Plan-level branching (`If`, `Match`, workflow `Sequence`) | No | No | No | Yes |
| Plan-level loops (`While`, `RepeatUntil`, `ForEach`) | No | No | No | Yes |
| Plan-level variables or registers | No | No | No | Yes |

#### What this means for implementability

- **Terminus2** looks implementable in the new architecture if the runtime
  exposes bounded effects such as `RunSubcall`, `RewriteTranscript`, and richer
  loop-visible operation results, plus a future PTY-native capability layer.
- **Terminus Kira** also looks implementable in principle, but it depends more
  heavily on PTY-native runtime support and multimodal/runtime-native
  observation flows.
- **Codex**, **OpenCode**, **OpenClaw**, and **Gastown** all point toward
  richer runtime capabilities and stateful orchestration boundaries, not toward
  embedding a mini interpreter in the loop return type.

This is the key boundary to preserve: the loop should remain declarative, but it
does not need to stay tiny. It should become richer by adding runtime-native
effects and typed state, not by adding generic workflow control flow.

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
