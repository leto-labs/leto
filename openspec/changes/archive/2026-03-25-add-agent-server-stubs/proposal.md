# Proposal: add-agent-server-stubs

## Why

The repo now has a v2 runtime stack centered on `agent-runtime`, `agent-store`,
and `agent-core`, but it still lacks the hosted server boundary that makes a
future remote `AgentCore` implementation possible.

Two issues need to be addressed together:

1. `agent-core` is currently shaped as one concrete local type rather than as a
   shared consumer-facing boundary that can hide locality.
2. The new server work should not revive the old `brain-server` / `BrainApi`
   direction. It should instead host the v2 `AgentCore` surface and expose both
   the canonical API and an OpenCode compatibility adapter.

This change establishes the first hosted `agent-server` crate, locks the
canonical server namespace under `/v1`, and makes the OpenCode-compatible
surface explicitly secondary under `/v1/compat/opencode`.

## What Changes

This change:

1. modifies `agent-core` so `AgentCore` becomes the shared trait boundary and
   `AgentCoreNative` becomes the current embedded implementation
2. adds a new `agent-server` capability and crate that hosts `Arc<dyn AgentCore>`
3. defines the canonical server surface under `/v1`
4. defines an OpenCode-compatible secondary surface under `/v1/compat/opencode`
5. keeps compatibility implementation intentionally partial, with explicit
   `501 not_implemented` responses where semantic mapping is still unresolved

## Impact

- Modified capability: `agent-core`
- Added capability: `agent-server`
- New workspace crate: `crates/agent-server`
- Compatibility reference input: `add-opencode-compat-surface` and the OpenCode
  published server docs at https://opencode.ai/docs/server/
