# ACP Protocol

## Summary

ACP is a JSON-RPC protocol for communication between a client and a coding
agent. Its center of gravity today is editor-hosted agents launched as
subprocesses over stdio, but the official docs explicitly frame ACP as suitable
for both local and remote scenarios. The missing piece is transport maturity:
stdio is the current production path, while Streamable HTTP is still in draft.

For `brain`, the important conclusion is that ACP is already good enough for
editor interoperability right now, but should be designed so the same semantics
could later ride over richer transports.

## Core Model

ACP is session-centric. The lifecycle is:

1. `initialize`
2. `authenticate` if needed
3. `session/new` or `session/load`
4. repeated `session/prompt` turns with `session/update` notifications
5. `session/cancel` for interruption

That matches `brain` better than a stateless request/response API because
`brain` already thinks in projects, sessions, stored message history, and
turn-oriented event streams.

## Transport Reality

The official transport docs currently say:

- ACP uses UTF-8 JSON-RPC messages
- stdio is the defined transport in production
- agents and clients should support stdio whenever possible
- Streamable HTTP exists as a draft proposal in progress
- custom transports are allowed if they preserve ACP lifecycle and message rules

The important nuance is that the intro page also says ACP is suitable for both
local and remote scenarios, but "full support for remote agents is a work in
progress." In practice:

- local editor-hosted ACP is real now
- remote ACP should be treated as strategically important but not yet settled

## Session Semantics

The official session setup docs are stricter than a lot of internal "chat API"
designs:

- `session/new` includes `cwd` and per-session MCP servers
- `cwd` must be absolute
- the agent is expected to use that `cwd` regardless of the subprocess spawn
  location
- `session/load` requires replaying the full conversation back to the client as
  `session/update` notifications before responding

This matters because it implies ACP clients expect more than just "resume by
ID." They expect the agent to reconstruct the visible session history in-band.

For `brain`, that pushes toward:

- stable persisted session IDs
- a deterministic way to replay message history
- session loading as a first-class concept, not only "resume silently"

## Prompt Turn Expectations

ACP prompt turns are richer than token streaming alone. The protocol expects
agents to report:

- plan updates
- agent message chunks
- tool calls
- tool call updates
- permission requests
- final stop reasons

The protocol docs also assume the agent can continue iterating after tool
results, rather than treating a tool call as a separate API workflow.

This is a direct fit with `brain`'s iterative agent loop direction, but it also
shows where `brain` still has work:

- plan events should become first-class
- tool call lifecycle should be more structured
- permission and approval flows need explicit event modeling

## Client-Owned Capabilities

ACP is strongest where it gives the client meaningful ownership of the local
environment.

### Filesystem

The protocol defines `fs/read_text_file` and `fs/write_text_file`, and the docs
explicitly call out unsaved editor state as a motivation. That is a major
editor-specific advantage over a raw server API.

Implication for `brain`:

- an ACP surface should prefer client file access when the client advertises it
- `brain`'s native file tools still matter for non-ACP surfaces and for clients
  without FS support

### Terminal

The terminal methods let an agent create and manage shell commands in the
client's environment, stream output, wait for exit, kill the command, and embed
live terminal output into tool calls.

Implication for `brain`:

- ACP is not just a prettier chat shell
- if `brain` ignores ACP terminal support, it loses one of the main reasons to
  use ACP in editor clients

### Permissions

The prompt-turn docs assume the client can present permission choices back to
the user. That is exactly the right separation for editor UX: the client owns
the prompt/approval interface, while the agent owns the action semantics.

Implication for `brain`:

- tool approval should be modeled at the runtime level, then surfaced naturally
  through ACP permissions in ACP clients and through `BrainServer` in `brain`'s
  own UIs

## Modes And Config

ACP sessions can expose:

- available modes
- current mode
- config options
- unstable model selection

This is useful for `brain` because it gives a protocol-native place for:

- plan mode vs execution mode
- model picker support
- reasoning level or verbosity choices
- future approval policies

The important practical update is that ACP config options are now the right
session-level control surface for settings like reasoning or fast-mode style
knobs. That does **not** mean every client already exposes them well. The
protocol and the client UX have to be treated as separate questions.

For the current repo decision-making path:

- ACP-the-protocol already supports session modes and config options cleanly
- some clients, such as Codex ACP, use that surface directly
- the inspected Nori ACP path appears narrower and more model-centric
- if richer ACP-native session settings matter in the main TUI, extending the
  client is cleaner than forcing `agent-acp` to encode client-specific bridge
  behavior

See also:

- [`ClientCapabilityMatrix.md`](ClientCapabilityMatrix.md)
- [`Nori.md`](Nori.md)

The protocol does not force a rich mode system immediately. An agent can start
small and advertise more later.

## Maturity Boundaries

What looks mature today:

- stdio subprocess launch
- initialize/auth/session/prompt/cancel flow
- file and terminal client capabilities
- session modes and config options
- editor-hosted UX patterns

What still looks in-progress:

- remote/hosted transport convergence
- Streamable HTTP
- some unstable session features such as list/fork/resume/close

## What This Means For `brain`

- `brain` should implement ACP now for editor-hosted stdio clients.
- `brain` should keep `BrainServer` for native HTTP/SSE and non-ACP surfaces.
- `brain` should align its runtime event model with ACP concepts instead of
  forcing lossy adapter logic later.
- `brain` should not wait for Streamable HTTP to get ACP value, but it should
  avoid hard-coding assumptions that only stdio matters.

## Key Sources

- Official transport docs: <https://agentclientprotocol.com/protocol/transports>
- Official intro: <https://agentclientprotocol.com/get-started/introduction>
- Official Rust library docs: <https://agentclientprotocol.com/libraries/rust>
- Session setup docs: [`docs/protocol/session-setup.mdx`](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/session-setup.mdx)
- Prompt turn docs: [`docs/protocol/prompt-turn.mdx`](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/prompt-turn.mdx)
- Filesystem docs: [`docs/protocol/file-system.mdx`](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/file-system.mdx)
- Terminal docs: [`docs/protocol/terminals.mdx`](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/terminals.mdx)
- Session modes docs: [`docs/protocol/session-modes.mdx`](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/session-modes.mdx)
- Rust SDK agent trait: [`src/agent-client-protocol/src/agent.rs`](../../../repocache/agentclientprotocol/rust-sdk/src/agent-client-protocol/src/agent.rs)
- Rust SDK example agent: [`src/agent-client-protocol/examples/agent.rs`](../../../repocache/agentclientprotocol/rust-sdk/src/agent-client-protocol/examples/agent.rs)
