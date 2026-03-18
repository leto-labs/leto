# Design: add-acp-client-surface

## Decision: Stage One Is A Standalone Mock ACP Agent

The first ACP milestone should prove protocol correctness and editor
interoperability, not runtime integration.

The initial `brain-acp` crate should therefore be a standalone mock ACP agent
with no dependency on `brain-core`, `brain-server`, or the rest of the `brain`
runtime stack.

This stage exists to answer a narrow question:

- can `brain` implement ACP over stdio in a way that works cleanly with a
  terminal-native ACP client first, and then with a richer ACP TUI such as
  Nori?

It does not attempt to answer the harder follow-up questions yet:

- how ACP sessions map onto real `brain` sessions
- how `Brain.turn()` events map onto ACP `session/update`
- how approvals, filesystem, and terminal capabilities should be integrated

## Decision: Dedicated brain-acp Crate

The ACP surface should still live in a dedicated workspace crate,
tentatively `brain-acp`.

Even in mock form, ACP introduces concerns that do not belong in other crates:

- JSON-RPC request/response handling
- stdio transport lifecycle
- ACP capability negotiation
- ACP session/update payload mapping
- ACP-specific tests and fixtures

Keeping those concerns isolated gives the project a clean place to evolve from
mock behavior into a real runtime-backed ACP surface later.

## Decision: Use The Official ACP Rust SDK For Transport And Schema

The first-stage `brain-acp` crate should use the official ACP Rust SDK for the
protocol layer rather than hand-rolling JSON-RPC framing and method dispatch.

That gives the PoC:

- official ACP request and response types
- a standard `Agent` trait surface
- stdio connection plumbing through `AgentSideConnection`
- less protocol boilerplate to maintain

The SDK choice should be treated as an ACP implementation detail inside
`brain-acp`, not as a runtime abstraction for the rest of the workspace.

## Decision: Stdio First

The PoC should support ACP over stdio only.

That is the correct first transport because:

- ACP clients in practice launch agents as subprocesses over stdin/stdout
- stdio is sufficient to validate the ACP layer with a real editor client

Draft streamable HTTP support should remain out of scope for this first stage.

## Decision: Terminal-First Manual Validation

The first manual validation target should be a terminal-native ACP client rather
than a full editor integration.

That is the right operational default because:

- a terminal client is easier to launch repeatedly against local `brain acp`
- it is easier to drive through background shell sessions during development
- it exercises the ACP transport and lifecycle without pulling editor-specific
  UI assumptions into the first milestone

For this change, `acpx` should be treated as the baseline manual validation
target when available, and Nori should be treated as the preferred richer TUI
validation client for multi-turn UX checks. Editor smoke checks may still be
useful later, but they are not part of the completion criteria for the mock ACP
PoC.

## Decision: Mock Responses Are Acceptable In Stage One

The first-stage `brain-acp` crate should return hard-coded or fixed data where
that simplifies ACP validation.

Acceptable mock behavior includes:

- fixed agent metadata during initialization
- a fixed or in-memory list of sessions
- fixed or synthetic session history for `session/load`
- echoed or canned prompt responses for `session/prompt`
- fixed mode or config option data when needed for UI validation

The goal is to demonstrate the ACP lifecycle, not to simulate the full `brain`
runtime with high fidelity.

## Decision: Implement A Broad Mock ACP Surface

The PoC should implement a broad slice of the ACP agent surface rather than
stopping at the smallest Zed-compatible subset.

The mock implementation should cover:

- initialization and capability negotiation
- authentication
- session creation, loading, listing, and replay
- prompt turns with streamed updates and cancellation
- session modes
- session config options
- extension methods and notifications
- unstable session features from the official SDK when enabled for the mock
  crate, such as model selection, session resuming, and session closing
- agent-initiated use of client-owned ACP requests through fixed mock commands,
  including filesystem, permission, and terminal requests

This remains a mock design goal, not a runtime-integration goal. The broad
surface exists to exercise protocol handling and client interoperability, not to
freeze the final `brain` runtime contract.

The ACP layer should still advertise only the capabilities it actually supports.
Unsupported methods should continue to return protocol-correct errors rather
than partially simulating deeper runtime behavior.

## Decision: Exercise Client-Owned ACP Requests Through Fixed Mock Commands

The mock should expose a small set of deterministic `mock:`-prefixed commands whose only
purpose is to trigger client-owned ACP requests from the agent side.

Examples include:

- `mock:create-plan`
- `mock:summarize-session`
- `mock:think <topic>`
- `mock:search <query>`
- `mock:fetch <resource>`
- `mock:edit-file <absolute-path>`
- `mock:delete-file <absolute-path>`
- `mock:move-file <absolute-from> <absolute-to>`
- `mock:read-file <absolute-path>`
- `mock:write-file <absolute-path> <content>`
- `mock:request-permission`
- `mock:terminal <command...>`
- `mock:terminal-kill <command...>`

This is the cleanest way to validate the full ACP round trip in a real client
without coupling the mock to `brain-core` tools or building a fake planning or
tool system around those requests.

Normal prompt turns should remain simple echo behavior. Synthetic plans should
only be emitted through explicit mock commands such as `mock:create-plan`, and
the mock should prefer deterministic tool-call updates over real side effects
when a richer ACP UI signal is more important than true filesystem mutation.
The same rule applies to higher-level updates: reasoning chunks, search/fetch,
and patch-style file operations should be explicit `mock:` commands rather than
ambient behavior on ordinary prompts.

## Decision: No Runtime Contract Yet

The mock ACP stage should avoid freezing a runtime contract before the protocol
surface is tested in practice.

That means this change should not promise:

- `brain-core` integration
- `BrainServer` alignment requirements
- event model parity
- real persistence semantics
- tool execution through ACP client methods

Those belong in a follow-up change once the standalone ACP crate has proven the
transport and request lifecycle.

## Planned Follow-Up

After the mock ACP agent is proven in `acpx` and exercised through Nori, a
later change should define:

- the runtime integration boundary inside `brain-acp`
- real session persistence and replay
- mapping from `brain` events to ACP session updates
- use of ACP client-owned capabilities such as filesystem, terminal, and
  permissions
- any dedicated modes such as `architect` or richer config options

That later work should remain ACP-native in `brain-acp`, not an ACP-over-HTTP
or ACP-over-`BrainServer` bridge.

`BrainServer` remains strategically important for first-party clients and for
use cases where ACP is not the best fit, but it is parallel to the ACP runtime
work rather than the substrate beneath it.
