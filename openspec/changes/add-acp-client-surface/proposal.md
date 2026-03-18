# Proposal: add-acp-client-surface

## Why

`brain` currently has the beginnings of two different ACP stories:

1. `enrich-event-model` and `add-brain-cli` already point toward ACP-aligned
   events and a future `brain acp` mode.
2. `add-server-architecture` frames ACP as "optional, later" and primarily as
   a thin bridge over `BrainServer`.

After deeper research, that split no longer looks like the right strategic
default.

As of 2026-03-18:

- the official ACP docs describe both local and remote scenarios
- ACP has an official registry and public client ecosystem
- Zed and JetBrains now integrate the registry directly
- there is a real VS Code ACP client extension
- serious coding-agent products such as Codex, Cline, OpenCode, and OpenClaw
  now expose ACP entry points or bridges

This changes the product calculus for `brain`.

ACP is no longer only an optional compatibility shim. It is the most credible
path to reaching existing editor and IDE shells without first building a custom
TUI, VS Code extension, and JetBrains integration.

## What

This change establishes ACP as a first-class client surface for `brain`,
alongside `BrainServer`.

Specifically:

- `brain` SHALL gain a first-class ACP agent surface over stdio for editor and
  ACP-compatible client interoperability.
- `BrainServer` SHALL remain a first-class native HTTP/SSE surface for `brain`'s
  own TUI, web, and non-ACP clients.
- The ACP surface SHALL be built over the same runtime concepts as
  `BrainServer`, rather than being treated purely as a disposable bridge.
- The ACP design SHALL use ACP client capabilities such as filesystem,
  terminal, and permission requests when available.
- Streamable HTTP SHALL be tracked as an important future ACP transport, but it
  SHALL NOT block initial ACP support.

## Why This Direction Is Better

- It keeps `brain` focused on runtime quality instead of custom UI surface area.
- It gives `brain` a path into Zed, JetBrains, VS Code ACP, Neovim ACP clients,
  and emerging non-editor clients.
- It preserves `BrainServer` as the best native surface for `brain`-owned
  experiences and generic HTTP consumers.
- It avoids designing `brain`'s event model and session model in ways that later
  force a lossy ACP adapter.

## Impact

- New capability: `brain-acp`
- Modified capability: `brain-server`
- Planning implication: ACP should no longer be treated only as a later thin
  bridge in architecture discussions
- Runtime implication: event, session, approval, filesystem, and terminal
  semantics should be designed to map cleanly to ACP and `BrainServer`

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
