## Why

The refactored `agent-core` / `agent-store` stack still lacked the project
bootstrap behavior that legacy `brain-core` depended on:

- project-local `.agents/config.toml` loading
- root `.agents/AGENTS.md` prompt fallback
- durable project-level prompt and loop defaults
- first-turn prompt seeding that preserves the original transcript baseline

Those gaps blocked local parity even after the provider-credential tranche was
completed.

## What Changes

- extend `agent-store` so projects persist normalized bootstrap defaults such as
  system prompt and default loop
- extend `agent-core` so project creation resolves project-local config and
  AGENTS fallback before persisting the project
- make session/runtime resolution honor project default loop selection
- seed the project prompt into the first turn transcript once so later project
  config edits do not rewrite historical prompt state

## Impact

- modified capability: `agent-store`
- modified capability: `agent-core`
- no standalone `brain-config`-style public crate is reintroduced
- project-local config is restored; legacy user-global `~/.brain/config.toml`
  layering is intentionally not part of this tranche
