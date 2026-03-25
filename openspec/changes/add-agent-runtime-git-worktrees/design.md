# Design: Runtime-Managed Git Worktrees

## Core Decision

Add git worktrees as a first-class runtime resource aimed at child-runtime
isolation.

This change is intentionally shaped more like PTY management than like a
general repository SDK:

- the runtime owns worktree identity, lifecycle, registry state, and events
- callers use typed runtime operations instead of shelling out to `git`
- child runtimes may bind to an existing worktree explicitly
- spawn does not auto-create worktrees in v1

The scope is deliberately bounded:

- no generic multi-backend worktree driver trait yet
- no full repository administration surface
- no persistence or restart recovery in v1
- no automatic bare-repo or clone topology like GasTown

## Research-Grounded Direction

The external references suggest three distinct patterns:

- OpenCode proves the value of a canonical project identity plus managed
  worktree paths and explicit create/remove flows.
- GasTown proves that worktrees are worth modeling as durable agent sandboxes
  with helper methods for add/remove/move/prune/list, but it also carries a much
  larger operational model built around bare repos and rig infrastructure.
- Codex and ZeroClaw show that worktree awareness or safe git commands alone are
  not enough for runtime-managed child isolation.

For `agent-runtime`, the right first step is:

- adopt the first-class resource model
- avoid the heavier bare-repo architecture
- keep the initial backend native and minimal

## Runtime Shape

The runtime gains a flat shared worktree registry, analogous to PTY tracking.
Each worktree has:

- stable `WorktreeId`
- source repository root
- resolved worktree path
- branch name and optional start ref metadata
- owner/runtime provenance
- attachment state describing which runtime, if any, is currently bound
- lifecycle status and last runtime-visible error, when relevant

The registry remains the source of truth. Individual session snapshots only
cache visible worktree state and an optional bound worktree reference for the
current runtime.

## Public Surface

This change adds typed runtime-facing structures and commands for:

- `CreateWorktreeRequest`
- `RemoveWorktreeRequest`
- `BindWorktreeRequest`
- `UnbindWorktreeRequest`
- `WorktreeState`
- `WorktreeId`

`SpawnRequest` gains an optional `worktree_id`. If present, the child runtime
starts bound to that existing worktree. If absent, spawn behavior is unchanged.

The runtime also emits worktree lifecycle events:

- created
- bound
- unbound
- removed
- failed

Provider-visible native tools should mirror the same bounded surface:

- create worktree
- list worktrees
- get worktree
- remove worktree
- bind worktree
- unbind worktree

## Backend Behavior

The initial implementation stays native-first and uses the git CLI from inside
`agent-runtime`.

Worktree creation rules:

- caller provides a `repo_root`
- caller may optionally provide an explicit target directory
- if no target directory is provided, the runtime allocates one under a new
  managed worktree root configured in runtime config
- if neither an explicit directory nor a managed worktree root is available,
  creation fails with a typed runtime error

Git operations in v1 are intentionally minimal:

- create via `git worktree add`
- inspect/list via `git worktree list --porcelain`
- remove via `git worktree remove`

The runtime does not manage separate bare repositories, mirrors, or clone
topologies. It operates against an existing git repository root.

## Binding Semantics

Binding is explicit and exclusive in v1:

- a runtime may have zero or one bound worktree
- a worktree may be bound to zero or one runtime at a time
- binding an already bound worktree fails
- removing a bound worktree fails

This keeps ownership rules simple and avoids implicit coordination bugs between
parent/child runtimes.

## Local Execution Defaults

Bound worktrees affect runtime-owned local execution defaults.

When a runtime is bound to a worktree:

- PTY creation uses the bound worktree path when `cwd` is omitted
- future runtime-native local execution surfaces should inherit the same default

An explicit request `cwd` always wins over the bound-worktree default.

## Failure Model

Failures must be typed and non-corrupting:

- non-git repo roots are rejected cleanly
- git command failures do not partially register a worktree
- remove failures leave the existing registry entry intact
- bind/unbind failures do not mutate unrelated runtime state

## Non-Goals For V1

This change does not include:

- auto-provisioning a new worktree as part of spawn
- shared or multi-attach bindings
- move/reset/prune operations
- persistence across process restarts
- alternate backends or a dedicated worktree driver trait
