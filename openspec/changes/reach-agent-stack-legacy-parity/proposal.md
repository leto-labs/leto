## Why

The refactored `provider-*` and `agent-*` stack is now the intended long-term
architecture, but several important user-visible behaviors still only exist in
legacy `brain-*` crates.

Today, the new stack already provides stronger standalone provider and runtime
primitives, but it does not yet reach full legacy parity for:

- advanced loop strategies beyond `SimpleLoop`
- project bootstrap and prompt/loop defaults
- terminal-session semantics
- fully real compatibility routes in `agent-server`

As long as those gaps remain, real composition still depends on legacy glue and
the migration to the refactored stack remains incomplete.

## What Changes

- define full parity as restoring all legacy user-visible behavior on the
  refactored stack rather than preserving long-term dependence on `brain-*`
- extend `agent-store` and `agent-core` to restore legacy bootstrap,
  configuration, and remaining assembly behavior
- add parity requirements for advanced `agent-loops` strategies equivalent to
  legacy `robust`, `terminus2`, and `terminus-kira`
- add parity requirements for `agent-tools` terminal-session behavior
- require `agent-server` compatibility routes to use real auth and PTY-backed
  implementations rather than placeholder responses

## Impact

- new parity-program OpenSpec change covering the remaining `agent-store`,
  `agent-core`, `agent-loops`, `agent-tools`, and `agent-server` parity work
- the completed provider-credential tranche is tracked separately so its
  canonical specs can be archived now
- future implementation work will include behavior additions and some breaking
  cleanups in v2 configuration and bootstrap surfaces
- legacy `brain-*` crates remain the behavioral baseline until the parity tasks
  are implemented and verified
