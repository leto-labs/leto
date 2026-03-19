# Proposal: refactor-brain-types-trait-modules

## Why

`brain-types` already exposes several core abstractions from dedicated modules
such as `provider.rs` and `transport.rs`, but `Tool`, `Store`, and `AgentLoop`
still live together in a generic `traits.rs` file.

That layout makes the crate inconsistent and forces unrelated abstractions to
share a catch-all module. Splitting the remaining traits into dedicated files
improves discoverability and keeps the public API aligned with the rest of the
crate.

## What

- Move the `Tool` trait into `tool.rs` alongside `ToolDef` and `ToolCall`
- Add `store.rs` for `ProjectStore`, `SessionStore`, `MessageStore`,
  `CredentialStore`, and the composite `Store` trait
- Add `agent_loop.rs` for `EventStream` and `AgentLoop`
- Update `brain-types` exports so callers can import these APIs from dedicated
  modules while keeping the existing root-level re-exports intact
- Add a public API test that locks in the new module layout

## Impact

- **Modified crate**: `brain-types`
- **Public API change**: dedicated submodules become available for store and
  agent loop traits
- **Compatibility**: root-level imports such as `brain_types::Store` and
  `brain_types::AgentLoop` remain available
