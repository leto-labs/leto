# Proposal: add-session-loop-config

## Why

`brain-acp` now exposes ACP session config for `model` and `thought_level`, but
the runtime already has another real session-level control that ACP clients
cannot edit yet: `loop_name`.

Loop selection already exists in shared config/session types and in the runtime
loop registry. Exposing it through ACP `configOptions` is a natural extension of
the current architecture and lets clients like Nori surface loop selection
without any `brain`-specific logic.

## What Changes

- add a runtime helper for persisting session loop overrides
- expose `loop` as an ACP session config option in the real backend when more
  than one loop is registered
- add a multi-value `loop` config option to the mock ACP agent for client UX
  validation
- keep ACP session modes out of scope

## Impact

- extends the public `BrainRuntime` helper surface with loop mutation
- extends `brain-acp` session config with one additional generic select option
- keeps real backend UX clean by omitting `loop` when only one loop is available
