## Overview

This change restores bootstrap/default parity without recreating the legacy
crate boundaries.

The chosen architecture is:

- `agent-store` persists normalized project defaults only
- `agent-core` owns filesystem bootstrap from `.agents/config.toml`
- `agent-core` applies root `.agents/AGENTS.md` only when no explicit prompt is
  configured
- transcript history, not mutable project config, remains the source of truth
  for the original prompt after the first turn

## Design Decisions

### No standalone `brain-config` replacement

The old `brain-config` crate mixed raw filesystem parsing with layering policy.
For v2, the only needed behavior is project-local bootstrap, and that belongs
inside `agent-core` because it is part of application composition rather than a
reusable provider/runtime substrate.

### Normalize before persisting

`agent-store` should not know about TOML files or AGENTS file loading. It
stores only the resolved project defaults:

- system prompt
- default loop
- default provider/model
- runtime-native request and hardening defaults

### Preserve historical prompt state

The project prompt is injected into the first turn transcript when no explicit
system or developer message is already present. After that, later project prompt
changes do not rewrite earlier transcript history.
