# Proposal: add-server-architecture

## Why

This planning change no longer matches the current stack and should not stay in
the active queue as part of legacy migration cleanup.

The remote-runtime idea was originally framed around `BrainRuntime`,
`BrainRuntimeRemote`, and future `brain-server` work. The codebase has since
moved to the `agent-core` / `agent-server` / `agent-core-remote` architecture,
and the legacy `brain-server` direction is no longer the target design.

Keeping this change active now creates false future work for legacy crates that
are meant to be deleted.

## What Changes

This change is being retired rather than implemented.

The architectural decision is:

1. treat `agent-core-remote` plus `agent-server` as the current remote/server
   direction
2. stop carrying forward a future `brain-server` / `BrainRuntimeRemote` plan as
   active work
3. keep this change only as historical context for an abandoned legacy-centered
   architecture

## Impact

- No implementation change: this proposal is archived as superseded
- Clarifies direction: remote/server work is owned by `agent-server` and
  `agent-core-remote`
- Removes a stale future-architecture blocker from the active legacy-migration
  queue
