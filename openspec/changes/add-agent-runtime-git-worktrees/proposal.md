# Proposal: Add Runtime-Managed Git Worktrees

## Why

`agent-runtime` already has first-class child runtimes, cross-runtime
messaging, waiting, and runtime-managed PTYs, but local execution still has no
repo-aware isolation model beyond an optional PTY `cwd`.

That gap matters for coding-agent behavior:

- child runtimes cannot be cleanly isolated onto their own git-backed working
  trees
- the runtime has no stable identity or lifecycle for a managed worktree
- local execution cannot inherit a runtime-owned repository sandbox by default
- loops and outer transports must fall back to ad-hoc shell conventions instead
  of typed runtime state

Research across other agent runtimes shows a useful split:

- OpenCode and GasTown treat worktrees as first-class managed sandboxes
- Codex is worktree-aware for trust and git introspection, but does not expose a
  full worktree lifecycle
- ZeroClaw exposes safe git operations inside an existing workspace, not
  worktree provisioning

For `agent-runtime`, the immediate need is not a generic repo-management
framework. The immediate need is a runtime-owned worktree resource for child
isolation.

## What Changes

- add a first-class git worktree subsystem to `agent-runtime`
- expose typed runtime operations for create/list/get/remove/bind/unbind
  worktree lifecycle
- allow child spawn to reference an existing worktree id
- make a bound worktree the default local execution directory when runtime-owned
  local execution omits an explicit `cwd`
- expose provider-visible native worktree tools mirroring the typed runtime
  actions
- keep v1 native-first and git-CLI-based, without adding a new backend trait
- keep v1 in-memory only, without persistence or restart recovery

## Impact

- modifies `agent-runtime`
- extends the public runtime command/state/event surface
- gives loops and transports a typed way to manage git-backed child sandboxes
  without inventing an external orchestration layer
