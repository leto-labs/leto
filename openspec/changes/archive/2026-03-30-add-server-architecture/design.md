# Design: add-server-architecture

## Decision: Retire The BrainRuntimeRemote Planning Track

This change is superseded by the current architecture.

The relevant design correction is:

- remote/server architecture should be described through `agent-server` and
  `agent-core-remote`
- the legacy `brain-server` / `BrainRuntimeRemote` direction should not remain
  as active future work

## Decision: Keep This Change Only As Historical Context

The preserved context explains an older planning frame, but it should not drive
new implementation or migration-closeout work.
