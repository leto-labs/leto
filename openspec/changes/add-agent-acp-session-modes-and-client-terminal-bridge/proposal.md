# Proposal: add-agent-acp-session-modes-and-client-terminal-bridge

## Why

`agent-acp` can already drive basic ACP sessions, but it still falls short of
the richer session-control shape that Codex-style ACP clients expect:

- the real backend does not expose ACP session modes
- the real backend does not advertise ACP available commands
- the default local launch path still falls back to the legacy `.brain` store
  root, which causes opaque startup failures on machines with incompatible
  legacy state
- the checked-out Nori fork still rejects ACP terminal client methods, which
  blocks the mock and real ACP terminal flows from working through the TUI

These gaps prevent the repository from using Nori as a serious ACP-first TUI
for the live backend.

## What Changes

This change adds the first concrete Codex-parity slice for ACP:

1. real `agent-acp` session modes backed by the runtime loop surface
2. real `agent-acp` session models plus `session/set_model`
3. real `agent-acp` available-command advertising plus lightweight slash
   command rewriting for the advertised commands
4. real `agent-acp` support for `session/resume`, `session/fork`, and
   `session/close`
5. broader real-backend session coverage: global session listing, better
   session metadata/config updates, and baseline `ResourceLink` prompt handling
6. a dedicated default ACP home/root path and clearer startup validation for
   incompatible legacy stores
7. Nori ACP client support for ACP terminal client methods

## Impact

- Modified capability: `agent-acp`
- Modified capability: `agent-acp-integration`
- Touches both the live backend and the checked-out Nori fork
