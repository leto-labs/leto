# Design: reach-agent-acp-legacy-parity

## Decision: Scope ACP Parity To Code-Backed Legacy Behavior

This change only tracks ACP behavior that still exists in legacy code and is
still consumed by the repo:

- direct real and mock ACP launch identities
- client-owned filesystem bridging
- session-model compatibility behavior
- repo-owned launchers and Harbor integration

OpenCode compatibility and MCP are explicitly out of scope because they were
never part of the legacy `brain-acp` baseline.

## Decision: Preserve Current Repo-Used ACP Behavior By Default

The default migration rule for this change is to preserve existing repo-used
ACP behavior unless there is an explicit retirement decision captured in the
spec.

That means:

- do not silently drop the mock validation lane
- do not silently drop Harbor's current session-model workflow
- do not silently replace ACP client-owned file operations with backend-host
  filesystem writes

## Decision: Keep The Migration Boundary On Agent ACP Surfaces

The target design belongs on current surfaces:

- `agent-acp` owns the backend and launch identities
- `repo-tooling` owns Harbor and repo launcher migration

The change should not add new future work to legacy `brain-*` capabilities.
