# Proposal

## Why

The new `agent-runtime` architecture successfully moved provider execution, tool
execution, waits, and child-runtime coordination out of loop implementations,
but two important hardening features from the legacy `RobustLoop` are still
missing as real runtime behavior:

- transcript compaction
- doom-loop detection / recovery

At the same time, the newer architecture should not regress into a runtime that
silently owns all orchestration policy. The runtime/loop boundary needs to stay
explicit so custom loops can choose their own recovery behavior.

## What

- add transcript compaction support to `agent-runtime`
- add runtime-backed repeated-tool doom-loop detection to `agent-runtime`
- add a loop-visible steering decision so loops can react to doom-loop warnings
- update `agent-loops::SimpleLoop` to use runtime advice for compaction and
  one-shot doom-loop steering
- document the runtime-versus-loop design rule under `docs/architecture/`

## Impact

- modifies `agent-runtime`
- modifies `agent-loops`
- adds architecture documentation for the runtime/loop boundary
