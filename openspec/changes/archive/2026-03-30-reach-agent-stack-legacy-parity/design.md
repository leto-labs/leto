## Overview

This change defines a parity program, not a compatibility shim plan.

The target state is:

- `provider-*` and `agent-*` are the canonical implementation path
- legacy `brain-*` crates serve only as the behavioral reference
- missing legacy behavior is ported into the new stack in subsystem-appropriate
  places

The design rule is to preserve stronger v2 architecture where it already
surpasses legacy, while restoring every missing externally observable behavior.

## Legacy Baseline

The parity baseline comes from the combined behavior of:

- legacy OpenAI API-key and OAuth providers
- legacy credential-pool strategy and health behavior
- legacy advanced loops (`robust`, `terminus2`, `terminus-kira`)
- legacy terminal-session tool behavior
- legacy ACP, CLI, and server surfaces that actually existed in code

For parity purposes, "full parity" means user-visible and integration-visible
behavior, not literal type or module duplication.

The baseline explicitly excludes:

- OpenCode compatibility routes, because legacy `brain-server` never
  implemented them
- MCP, because legacy `brain-*` crates never implemented it

## Subsystem Mapping

### Provider and auth

The provider-credential tranche has been split into a focused change so it can
be archived independently of the remaining parity program. This umbrella change
now only tracks the remaining parity work that was not completed by the shared
provider credential pool, the standalone OpenAI OAuth surface, and the
store-backed activation layer in `agent-core`.

### Core composition

The bootstrap/defaults tranche has been split into a focused change so it can
be archived independently of the remaining parity program. Any later
`agent-core` work tracked by this umbrella change should be limited to assembly
needs that are downstream of the advanced loops and terminal-session surfaces.

### Loops and terminal behavior

`agent-loops` must expose advanced loop strategies with legacy-equivalent
outcomes, while continuing to use the runtime-native v2 control surface.
Terminal-session parity for those loops should be delivered through
`agent-runtime` PTY/session surfaces and loop-private orchestration rather than
through a duplicate public terminal tool wrapper.

## Sequencing

Implementation should proceed in dependency order:

1. advanced loop parity
2. runtime-native terminal-session parity

This order minimizes rework because the later loop behaviors depend on the
earlier runtime and tooling layers. Hosted compatibility work is a separate
track and not part of this legacy-parity change.

## Tradeoffs

- Do not reintroduce a monolithic legacy-style catch-all crate to achieve
  parity; extend the existing v2 boundaries instead.
- Do not declare parity complete based only on internal runtime capability.
  The higher-level assembly and the legacy user-facing surfaces that actually
  existed in code must also match legacy behavior.
- Do not treat current `agent-server` OpenCode compatibility work as a proxy
  for legacy parity when that surface never existed in `brain-server`.
