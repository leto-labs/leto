## Why

The refactored `provider-*` and `agent-*` stack is now the intended long-term
architecture, but several important user-visible behaviors still only exist in
legacy `brain-*` crates.

Today, the new stack already provides stronger standalone provider and runtime
primitives, but it does not yet reach full legacy parity for:

- advanced loop strategies beyond `SimpleLoop`
- terminal-session semantics
- fully real compatibility routes in `agent-server`

As long as those gaps remain, real composition still depends on legacy glue and
the migration to the refactored stack remains incomplete.

## What Changes

- define full parity as restoring all legacy user-visible behavior on the
  refactored stack rather than preserving long-term dependence on `brain-*`
- add parity requirements for advanced `agent-loops` strategies equivalent to
  legacy `robust`, `terminus2`, and `terminus-kira`
- add parity requirements for `agent-tools` terminal-session behavior
- require `agent-server` compatibility routes to use real auth and PTY-backed
  implementations rather than placeholder responses

## Impact

- active parity-program OpenSpec change now covering the remaining
  `agent-loops`, `agent-tools`, and `agent-server` parity work
- the completed provider-credential and bootstrap/defaults tranches are tracked
  separately so their canonical specs can be archived independently
- future implementation work will focus on advanced loop, terminal-session, and
  hosted compat behavior
- legacy `brain-*` crates remain the behavioral baseline until the parity tasks
  are implemented and verified
