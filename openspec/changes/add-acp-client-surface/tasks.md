# Tasks: add-acp-client-surface

## Research And Planning

- [x] Produce `docs/acp/` with protocol, registry, client, and adapter research
- [x] Add ACP-critical research repos to `repocache/repocache.json`
- [x] Create the OpenSpec proposal, design, tasks, and spec deltas for ACP as a first-class surface

## Mock ACP PoC Design

- [x] Create a dedicated `brain-acp` crate for ACP protocol handling and stdio transport
- [x] Use the official ACP Rust SDK for the first-stage protocol implementation
- [x] Expose the crate initially through `brain acp`, keeping standalone binary packaging open for later
- [x] Define the mock ACP layer around capability-gated protocol coverage rather than runtime integration
- [x] Define a minimal mock data model for sessions, history, and prompt responses that requires no `brain-*` runtime dependencies

## Mock ACP Implementation

- [x] Implement ACP `initialize`, `authenticate` if needed, `session/new`, `session/load`, `session/prompt`, and `session/cancel`
- [x] Implement mock session replay for `session/load`
- [x] Implement the ACP prompt path with echoed or canned streamed responses
- [x] Implement mock ACP session listing, mode selection, config options, and extension methods
- [x] Implement unstable mock ACP session features that the SDK exposes for the PoC, including model selection, session resuming, and session closing
- [x] Implement fixed mock commands that trigger client-owned ACP filesystem, permission, and terminal requests
- [x] Implement explicit higher-level mock commands for reasoning chunks, search/fetch tool flows, and synthetic patch-style file operations
- [x] Keep the first-stage crate independent from `brain-core`, `brain-server`, and the current `brain` event model

## Verification

- [x] Add protocol-level tests for ACP handshake, session creation, prompt turns, and cancellation
- [x] Add replay tests for mock ACP `session/load`
- [x] Add tests for session listing, mode/config/model updates, and extension methods
- [x] Add tests for unstable session features enabled in the mock
- [x] Add tests for `mock:` prompt command triggers, including explicit mock planning and client-owned ACP requests rendered through tool-call updates
- [x] Add tests for higher-level `mock:` commands covering reasoning chunks, search/fetch tool kinds, and patch-style edit/delete/move flows
- [x] Add a manual terminal-native ACP validation pass against local `brain acp`, using `acpx --agent "cargo run -q -p brain-cli -- acp"` as the canonical launcher
- [x] Add an opt-in `acpx` compatibility harness covering file, permission, terminal, session, and control-surface flows against local `brain acp`
- [x] Add a repo-owned Nori custom-prompt pack and symlink installer for rapid mock ACP validation in the TUI
- [x] Validate the change with `openspec validate add-acp-client-surface --strict`
