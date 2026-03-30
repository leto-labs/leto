# Design: add-opencode-compat-surface

## Decision: Retire The Legacy-Scoped Planning Frame

This change is superseded by the current codebase direction.

The important design correction is:

- OpenCode compatibility belongs to `agent-server`, not to `brain-server`
- OpenCode compatibility should not be used as a proxy for legacy
  `brain-*` migration completeness
- future compatibility planning should start from the current hosted stack,
  current compat routes, and current validation consumers

## Decision: Keep This Change Only As Historical Context

The preserved historical context is useful because it explains an abandoned
planning frame:

- treating compatibility as future `BrainRuntime` / `brain-server` work
- using that future work as a blocker on the legacy-migration story

That framing should no longer guide implementation or migration cleanup.
