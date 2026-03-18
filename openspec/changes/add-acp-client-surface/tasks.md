# Tasks: add-acp-client-surface

## Research And Planning

- [x] Produce `docs/acp/` with protocol, registry, client, and adapter research
- [x] Add ACP-critical research repos to `repocache/repocache.json`
- [x] Create the OpenSpec proposal, design, tasks, and spec deltas for ACP as a first-class surface

## Runtime And Surface Design

- [ ] Finalize the `brain-acp` crate or module boundary and feature gate strategy
- [ ] Decide the exact `brain` CLI surface for ACP startup (`brain acp` vs separate binary)
- [ ] Define how ACP session IDs map to `brain` session IDs and working-directory scoped project selection
- [ ] Define the canonical mapping from enriched `brain` events to ACP `session/update` notifications
- [ ] Decide which ACP optional capabilities are available in the first implementation and which remain unsupported

## ACP Implementation

- [ ] Implement ACP `initialize`, `authenticate` if needed, `session/new`, `session/load`, `session/prompt`, and `session/cancel`
- [ ] Implement ACP session replay for `session/load`
- [ ] Implement ACP permission requests backed by runtime approval events
- [ ] Implement ACP mode/config support only where backed by real runtime state
- [ ] Wire ACP filesystem methods into `brain` when ACP client FS capabilities are present
- [ ] Wire ACP terminal methods into `brain` when ACP client terminal support is present

## BrainServer Alignment

- [ ] Keep `BrainServer` as a first-class HTTP/SSE surface while ACP is added
- [ ] Ensure ACP and `BrainServer` share the same session and event semantics
- [ ] Avoid introducing ACP-only runtime concepts that cannot be represented through `BrainServer`

## Verification

- [ ] Add protocol-level tests for ACP handshake, session creation, prompt turns, and cancellation
- [ ] Add replay tests for ACP `session/load`
- [ ] Add tests for capability negotiation and unsupported optional features
- [ ] Add tests for ACP client filesystem and terminal integration paths
- [ ] Validate the change with `openspec validate add-acp-client-surface --strict`
