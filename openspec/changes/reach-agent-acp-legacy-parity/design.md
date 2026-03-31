# Design: reach-agent-acp-legacy-parity

## Decision: Scope ACP Parity To Code-Backed Legacy Behavior

This change now tracks the one ACP behavior that still exists in legacy code
and is not yet present on the live stack:

- client-owned filesystem bridging

OpenCode compatibility and MCP are explicitly out of scope because they were
never part of the legacy `brain-acp` baseline.

## Decision: Preserve Current Repo-Used ACP Behavior By Default

The default migration rule for the remaining work is to preserve ACP
client-owned filesystem behavior rather than silently falling back to
backend-host-only writes when a client exposes ACP filesystem capabilities.

The current Harbor path is already operational because the backend runs inside
the task container and can write directly into the task workspace. That does
not remove the need for explicit ACP client-owned filesystem mediation when a
client wants to own those file operations.

## Decision: Keep The Migration Boundary On Agent ACP Surfaces

The target design belongs on current surfaces:

- `agent-acp` owns the backend-facing client bridge behavior

The change should not add new future work to legacy `brain-*` capabilities.
