# Proposal: add-agent-runtime-and-loops

## Why

The workspace now has a standalone v2 `provider` crate with richer request,
event, and capability models, but the current loop/runtime stack is still bound
to legacy `brain-*` traits and event shapes.

We want to move up the stack using the same greenfield approach:

- no adapters to legacy loop/runtime crates
- no store integration in v1
- one reusable in-memory runtime crate built directly on `provider`
- one loop crate that proves the boundary with a simple tool loop

The initial runtime/loop split was a useful first step, but the original
per-turn `run(...)` loop trait still made advanced features awkward:

- steering could only be faked by cancellation and replay
- interruption had no safe-boundary semantics
- approval pause/resume had no first-class runtime vocabulary
- subagents had no child-session lifecycle model
- multiplexed outer inputs had no source-aware command surface

The next change refactors the greenfield crates into a bidirectional session
engine so later loops such as robust or terminal-oriented strategies can reuse
the same control and boundary machinery.

The follow-up problem is subagents. Advanced systems need child runtimes that
can be spawned, listed, observed, and messaged over time without turning each
runtime into a recursively-owned tree of other runtimes.

They also need runtime-native agent controls that can be exposed to the model
as tools without collapsing orchestration into the generic tool executor.

## What Changes

- refactor `agent-runtime` into a long-lived command-driven session engine
- add explicit session commands, control events, approval decisions, and
  session boundaries
- add a flat internal runtime registry that owns live runtime lookup, parent to
  child lineage, and parent-child message routing
- add typed agent actions for spawn, wait, input, interrupt, listing, and
  messaging
- add subagent spawn defaults that inherit the parent runtime’s effective
  provider/model/runtime settings unless explicitly overridden
- add provider-visible native agent tools such as `spawn_agent`,
  `message_agent`, `read_agent_mail`, `interrupt_agent`, `list_agents`, and
  `wait_agent`
- add direct-vs-mail cross-agent messaging and provider-visible child reports
- replace the per-turn loop trait with a decision-oriented loop strategy surface
- keep `agent-loops` focused on strategy implementations and port `SimpleLoop`
  to the new runtime
- add child-runtime request/event/types so the runtime can own delegation
  bookkeeping without coupling to legacy crates
- add OpenSpec capabilities for `agent-runtime` and `agent-loops`

## Impact

- New capability: `agent-runtime`
- New capability: `agent-loops`
- New crates: `agent-runtime`, `agent-loops`
- Workspace manifest updated to include the new crates
- Breaking greenfield API changes inside `agent-runtime` and `agent-loops`
