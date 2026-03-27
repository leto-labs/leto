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
- legacy project bootstrap and config defaults
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

`agent-core` becomes the canonical replacement for legacy runtime assembly. It
must still restore project bootstrap, persisted defaults, prompt/loop
selection, and any remaining assembly behavior so future CLI or hosted
consumers do not need legacy composition code.

### Loops and terminal behavior

`agent-loops` must expose advanced loop strategies with legacy-equivalent
outcomes, while continuing to use the runtime-native v2 control surface.
`agent-tools` must expose terminal-session behavior at the v2 tool layer rather
than leaving PTY behavior implicit inside runtime internals only.

### Hosted compatibility

`agent-server` must stop treating compat auth and PTY flows as placeholders.
Compatibility routes may adapt to refactored internals, but they must resolve
to real behaviors backed by the shared core/runtime stack.

## Sequencing

Implementation should proceed in dependency order:

1. project/store/bootstrap parity
2. any remaining provider assembly parity in `agent-core`
3. advanced loop parity
4. terminal-session parity
5. hosted compat-route parity

This order minimizes rework because the later user-facing surfaces depend on the
earlier auth, store, and composition layers.

## Tradeoffs

- Do not reintroduce a monolithic legacy-style catch-all crate to achieve
  parity; extend the existing v2 boundaries instead.
- Do not declare parity complete based only on internal runtime capability.
  The higher-level assembly and compat surfaces must also match legacy behavior.
- Do not preserve placeholder compat routes once their operations are part of
  the documented contract.
