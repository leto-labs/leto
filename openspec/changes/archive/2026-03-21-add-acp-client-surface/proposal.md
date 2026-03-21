# Proposal: add-acp-client-surface

## Why

ACP is still the right integration target, but the previously planned first step
is too large.

The earlier version of this change mixed together two goals:

1. prove that `brain` can speak ACP correctly to real external ACP clients
2. design a real runtime-backed ACP surface over `brain-core`

That is the wrong first slice. It forces protocol work, runtime integration,
event mapping, session persistence, and client capability handling into one
milestone.

For a first implementation pass, the lower-risk path is to build a standalone
mock ACP agent that demonstrates:

- stdio JSON-RPC transport
- ACP initialization and capability negotiation
- session lifecycle requests across the ACP surface
- prompt handling and streamed updates
- compatibility with a terminal-native ACP client and a richer multi-turn ACP
  TUI flow such as Nori

This gives the project a concrete ACP proof of concept without prematurely
coupling the protocol layer to `brain-core`, `brain-server`, or the current
event model.

## What

This change narrows the initial ACP work to a mock-first proof of concept.

Specifically:

- `brain` SHALL gain a dedicated `brain-acp` crate for ACP over stdio.
- The first-stage `brain-acp` implementation SHALL be standalone and SHALL NOT
  depend on `brain-core`, `brain-server`, or other `brain-*` runtime crates.
- The first-stage ACP implementation SHALL return fixed or hard-coded data for
  sessions, models, modes, config options, extension methods, and prompt
  responses as needed to prove protocol handling.
- The first-stage ACP implementation SHALL be sufficient for manual validation
  in a terminal-native ACP client, with Nori used for richer multi-turn TUI
  validation.
- The first-stage ACP implementation SHOULD implement the broad ACP agent
  request surface exposed by the official Rust SDK, using capability flags to
  advertise only the features it actually supports.
- A later change SHALL replace the mock behavior with a real runtime-backed ACP
  implementation that continues to live in `brain-acp`, once the protocol
  shape is proven.

## Why This Direction Is Better

- It isolates ACP transport and protocol work from runtime design decisions.
- It creates a fast path to verifying ACP interoperability in shells that are
  easier to drive locally than a full editor.
- It keeps the first ACP crate simple enough to build and test quickly.
- It reduces the risk of refactoring `brain-core` before the ACP surface is
  understood in practice.

## Impact

- New capability: `brain-acp`
- Planning implication: the first ACP milestone is a mock PoC, not a runtime
  integration
- Follow-up implication: runtime-backed ACP support should be proposed as a
  later `brain-acp` change after the mock layer is proven in a real client

## Research Inputs

The supporting research is captured in:

- `docs/acp/README.md`
- `docs/acp/Protocol.md`
- `docs/acp/Registry.md`
- `docs/acp/Zed.md`
- `docs/acp/JetBrains.md`
- `docs/acp/VSCode.md`
- `docs/acp/Cline.md`
- `docs/acp/Codex.md`
- `docs/acp/OpenClaw.md`
- `docs/acp/NonEditorClients.md`
- `docs/acp/AcpUi.md`
- `docs/acp/Nori.md`
- `docs/acp/TerminalValidation.md`
