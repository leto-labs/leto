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
- legacy user-facing composition surfaces

For parity purposes, "full parity" means user-visible and integration-visible
behavior, not literal type or module duplication.

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

### Hosted compatibility

`agent-server` must stop treating compat auth and PTY flows as placeholders.
Compatibility routes may adapt to refactored internals, but they must resolve
to real behaviors backed by the shared core/runtime stack.

## Sequencing

Implementation should proceed in dependency order:

1. advanced loop parity
2. runtime-native terminal-session parity
3. hosted compat-route parity

This order minimizes rework because the later user-facing surfaces depend on the
earlier runtime and tooling layers.

## Tradeoffs

- Do not reintroduce a monolithic legacy-style catch-all crate to achieve
  parity; extend the existing v2 boundaries instead.
- Do not declare parity complete based only on internal runtime capability.
  The higher-level assembly and compat surfaces must also match legacy behavior.
- Do not preserve placeholder compat routes once their operations are part of
  the documented contract.
