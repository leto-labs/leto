# Proposal: implement-agent-server-runtime

## Why

`agent-server` and `AgentCoreRemote` now exist, but the hosted server is still
only partially real:

- the canonical `/v1` API exists but is still minimal
- the OpenCode compatibility layer is still largely stubbed
- `/v1/compat/opencode/doc` is a placeholder instead of a useful contract
- browser-facing details like CORS are not yet aligned with real OpenCode
  usage

At the same time, the project now has two strong compatibility references:

1. the local OpenCode source in `repocache/anomalyco/opencode`
2. the exported OpenAPI contract in `openapi/opencode.json`

This makes it practical to turn `agent-server` into a true implementation
rather than a shape-only scaffold, and to use OpenCode's 1.3.2 contract as a
hard parity target for the compatibility surface.

## What

This change implements `agent-server` as a real hosted runtime with:

1. a fully-real canonical `/v1` API for `AgentCoreRemote`
2. a full OpenCode compatibility adapter under `/v1/compat/opencode`
3. generated compatibility `/doc` output derived only from the mounted compat
   router and Rust-side OpenCode contract metadata, with
   `openapi/opencode.json` used only in tests as the parity oracle
4. route inventory and parity tests driven by the OpenAPI file
5. server-local subsystems for OpenCode-only concerns such as PTY, workspace,
   MCP, TUI control, config UI state, and similar administrative surfaces
6. a structured `compat/opencode` module tree with route-family routers and a
   typed DTO layer covering the full set of body-bearing OpenCode compat
   operations
7. exact generated OpenAPI parity with OpenCode `v1.3.2`, except for the
   `/v1/compat/opencode` mount prefix

Compatibility work in this change is pinned to OpenCode release `v1.3.2`.
The local repocache clone should be refreshed and checked out to that tag
before final parity verification.

## Impact

- Modified capability: `agent-server`
- New code in `agent-server` for full compat/runtime behavior
- No change to the core `AgentCore` boundary
- Canonical `/v1` remains the primary remote runtime contract
- `/v1/compat/opencode` becomes a true secondary adapter rather than a stubbed
  placeholder
