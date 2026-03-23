# ACP Research

Source-first internal notes for Agent Client Protocol (ACP) and the clients,
adapters, and distribution mechanisms now growing around it.

This folder is intended to answer one concrete product question for `brain`:

> Should ACP become a core client surface for `brain`, alongside `BrainServer`,
> so we can reach existing editors and reuse existing UI shells instead of
> building custom ones first?

Short answer, as of 2026-03-18: yes.

ACP is already beyond a Zed-only experiment. The official protocol site now
describes both local and remote scenarios, the clients page includes editors,
desktop/web clients, notebooks, mobile apps, messaging bots, and connectors,
and both Zed and JetBrains have integrated the registry. There is also a real
VS Code ACP extension, plus production adapters such as `codex-acp`,
`openclaw acp`, and `cline --acp`.

## Why This Matters For `brain`

The strongest immediate value of ACP is not "one more protocol." It is a clean
separation of responsibilities:

- the client owns UI, editor state, permissions UI, filesystem access to
  unsaved buffers, terminals, logs, and agent selection
- the agent owns planning, tool orchestration, reasoning, session state, and
  model/provider routing

That split directly addresses the pain that has come up repeatedly in `brain`:

- building a good TUI is real product work
- building a VS Code extension is separate product work
- building a JetBrains integration is yet more product work
- ACP lets those shells already exist, while `brain` focuses on runtime quality

## Focus Areas

This research concentrates on the dimensions that matter most for `brain`:

- transport shape: stdio today, Streamable HTTP later
- session lifecycle: new, load, cancel, and resumability expectations
- client-owned capabilities: filesystem, terminals, permissions, logs
- session modes, model switching, and configuration options
- registry and install/distribution story
- editor and IDE expectations
- bridge implementations versus ACP-native runtimes
- feasibility outside editors: TUI, web, mobile, notebooks, messaging

## Index

- [`Protocol.md`](Protocol.md)
- [`Registry.md`](Registry.md)
- [`Zed.md`](Zed.md)
- [`JetBrains.md`](JetBrains.md)
- [`VSCode.md`](VSCode.md)
- [`Cline.md`](Cline.md)
- [`Codex.md`](Codex.md)
- [`OpenClaw.md`](OpenClaw.md)
- [`NonEditorClients.md`](NonEditorClients.md)
- [`Forge.md`](Forge.md)
- [`Nori.md`](Nori.md)
- [`AcpUi.md`](AcpUi.md)
- [`NeovimClients.md`](NeovimClients.md)
- [`TerminalValidation.md`](TerminalValidation.md)
- [`ClientCapabilityMatrix.md`](ClientCapabilityMatrix.md)

Current working set for this repo:

- `acpx` for baseline ACP compatibility validation
- Nori for multi-turn ACP TUI validation
- ACP UI as a useful standalone desktop reference

Current architectural reading of that working set:

- `acpx` is the best baseline probe for ACP control surfaces
- Nori is still the strongest TUI base
- if `brain` needs richer ACP-native session settings in a serious TUI, the
  cleaner next move is likely to extend or fork Nori rather than build a
  `brain-acp` bridge around client UX gaps

The remaining client notes are retained as research and comparison material,
not as the current implementation target.

## Executive Summary

- ACP is already a meaningful interoperability layer for coding agents, not just
  a draft idea. The protocol docs and Rust SDK are real, stable enough for
  current editor use, and structured around JSON-RPC session semantics that map
  well to `brain`'s existing session model.
- ACP is not strictly stdio-only in concept. The official docs explicitly say
  ACP is suitable for local and remote scenarios, while also stating that full
  remote support is still a work in progress. The transport page lists stdio
  today and Streamable HTTP as the draft path forward.
- Registry support changed the practical value of ACP. Zed and JetBrains now
  ship registry-based discovery and install flows, which means an ACP-speaking
  `brain` would not need per-client packaging forever.
- There is now a credible generic VS Code ACP client. It is not just a thin
  launcher; it has a chat panel, session tree, permission UI, terminal support,
  file-system handlers, traffic logging, and preconfigured ACP agent commands.
- Cline validates ACP as a portable editor shell around an existing product
  runtime. OpenClaw validates ACP as a bridge into non-editor environments and
  gateway-hosted runtimes. Codex validates ACP as a serious adapter surface for
  a full coding agent with approvals, terminals, auth, slash commands, and
  model/mode controls.
- The core architectural conclusion for `brain` is not "replace
  `BrainServer` with ACP." It is "make ACP and `BrainServer` dual primary
  surfaces." ACP is the best editor/IDE interop layer. `BrainServer` remains
  the cleanest foundation for `brain`'s own TUI, web, API, automation, and
  future non-ACP remoting.

## Ecosystem Snapshot

All rows below reflect the state observed on 2026-03-18.

| Area | Current state | Why it matters for `brain` |
| --- | --- | --- |
| Protocol | Official docs and SDKs, JSON-RPC session model, stdio production path, Streamable HTTP draft | Enough to implement a real ACP agent now without waiting for remote transport convergence |
| Registry | Official ACP registry live and integrated into Zed and JetBrains | ACP has distribution, not just transport |
| Editors | Zed and JetBrains have first-party ACP stories; VS Code has an extension; Neovim has ACP plugins | ACP can cover the editor surface faster than bespoke integrations |
| Adapters | Codex, Cline, OpenCode, OpenClaw and others expose ACP entry points or bridges | Strong evidence that serious agents see ACP as worth shipping |
| Non-editor clients | Official clients page already lists desktop/web apps, Jupyter, mobile, messaging, connectors | ACP is not editor-exclusive anymore, though editor UX is still the most mature |

## `brain` Baseline

`brain` already has several ACP-friendly traits and proposals:

- `BrainServer` gives `brain` a session-aware runtime boundary
- `enrich-event-model` already calls out ACP alignment as a goal
- `add-brain-cli` already has a placeholder task for `brain acp`
- the current conflict is architectural: one pending design treats ACP as a
  thin future bridge, while the event model and CLI tasks point toward
  first-class ACP support

ACP is therefore not a foreign direction for the repo. It is already present as
an unfinished architectural branch.

## Recommended Direction

### 1. Treat ACP and `BrainServer` as dual first-class surfaces

- ACP should be the primary interoperability layer for editors and any client
  that already speaks ACP.
- `BrainServer` should remain the primary `brain`-native surface for TUI, web,
  direct scripting, and future APIs that do not benefit from ACP.

### 2. Implement ACP directly, not only as an HTTP bridge

An HTTP bridge is still useful, especially if Streamable HTTP matures into the
dominant remote ACP transport. But the local-editor value of ACP is available
today over stdio, and clients already expect an ACP subprocess model.

### 3. Use ACP client capabilities aggressively where they improve UX

The biggest upside of ACP is that clients can give the agent:

- unsaved buffer reads
- file writes that show up naturally in the editor
- terminal execution with visible streamed output
- built-in permission prompts

`brain` should use those capabilities when present instead of pretending ACP is
only a chat transport.

### 4. Keep non-editor ACP ambitions experimental for now

The official clients page makes it clear that ACP can already reach mobile,
notebook, messaging, and connector scenarios. But the deepest maturity is still
in editor-hosted flows. `brain` should design with broader ACP futures in mind
without assuming every client category is equally mature today.

## Working Conclusion

ACP looks like the strongest current path for getting `brain` into high-quality
existing shells without committing to one UI stack first. It does not eliminate
the need for `BrainServer`; it strengthens the case for keeping runtime logic
separate from presentation and transport-specific UX.

The practical bet for `brain` is:

- ACP stdio now
- `BrainServer` in parallel
- event/session/tool semantics aligned so ACP and `BrainServer` are two views of
  the same runtime
- Streamable HTTP watched closely as the probable convergence point for richer
  remote ACP clients

## Primary Sources

- ACP introduction: <https://agentclientprotocol.com/get-started/introduction>
- ACP clients: <https://agentclientprotocol.com/get-started/clients>
- ACP transports: <https://agentclientprotocol.com/protocol/transports>
- ACP schema: <https://agentclientprotocol.com/protocol/schema>
- ACP Rust library docs: <https://agentclientprotocol.com/libraries/rust>
- Zed registry announcement: <https://zed.dev/blog/acp-registry>
- Zed external agents docs: <https://zed.dev/docs/ai/external-agents>
- JetBrains ACP docs: <https://www.jetbrains.com/help/ai-assistant/acp.html>
- JetBrains registry announcement: <https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/>
- ACP Rust SDK: [`repocache/agentclientprotocol/rust-sdk`](../../../repocache/agentclientprotocol/rust-sdk)
- ACP protocol repo: [`repocache/agentclientprotocol/agent-client-protocol`](../../../repocache/agentclientprotocol/agent-client-protocol)
- Codex ACP adapter: [`repocache/zed-industries/codex-acp`](../../../repocache/zed-industries/codex-acp)
- VS Code ACP extension: [`repocache/formulahendry/vscode-acp`](../../../repocache/formulahendry/vscode-acp)
- Cline ACP docs: [`repocache/cline/cline/docs/cline-cli/acp-editor-integrations.mdx`](../../../repocache/cline/cline/docs/cline-cli/acp-editor-integrations.mdx)
- OpenClaw ACP bridge docs: [`repocache/openclaw/openclaw/docs.acp.md`](../../../repocache/openclaw/openclaw/docs.acp.md)
