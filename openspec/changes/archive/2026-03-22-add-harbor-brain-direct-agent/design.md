# Design: add-harbor-brain-direct-agent

## Overview

This change started as a direct Harbor `brain` surface, but the final
implementation is broader. The Harbor path now depends on three architectural
decisions:

1. `brain exec` as a non-interactive one-shot runtime boundary
2. provider-declared OpenAI API-surface support, defaulting to Responses when
   supported
3. native Rust `ATIF` export, including runtime completion events and persisted
   trajectory state

The OpenSpec deltas need to reflect that broader shape so the archived specs
match the implementation.

## Key Decisions

### Keep one OpenAI-compatible provider family

We did not split OpenAI-compatible providers into separate provider identities
 such as `openai-responses` and `openai-chat-completions`.

Instead:

- provider presets declare supported API surfaces
- direct runs default to Responses when supported
- callers can still explicitly override the API surface

This keeps provider selection stable while making the modern API layer the
default path where available.

### Make ATIF a reusable Rust crate

ATIF is no longer just Harbor adapter output. It is a reusable Rust capability
with its own schema and validation surface.

That means:

- Harbor should consume native Rust-emitted `trajectory.json`
- ATIF behavior should not be documented only inside Harbor or CLI specs
- a dedicated `atif` capability spec is warranted

### Keep ATIF additive to runtime events

ATIF is not the universal runtime event language. The native operational event
stream remains the live orchestration surface.

When enabled, `Event::Atif(...)` adds transcript/export completion records to
that stream:

- `TrajectoryStarted`
- `StepCompleted`
- `FinalMetrics`
- `TrajectoryCompleted`

This preserves compatibility for runtime consumers that care about token/tool
events while enabling native transcript export.

### Persist trajectories separately from messages

We do not rebuild the full transcript from message replay plus the latest
config at the end of every turn.

Instead, the runtime persists session-scoped ATIF trajectory state and appends
new steps per turn. This preserves historical metadata such as:

- assistant `model_name`
- initial system prompt
- stable step ordering

That is why trajectory persistence belongs in store/runtime specs, not just in
the CLI export spec.

### Keep TurnDone terminal

ATIF completion records are emitted before `TurnDone`, and `TurnDone` remains
the terminal event of a successful turn.

This keeps the turn-stream contract coherent for interactive/runtime consumers
that stop on `TurnDone`.
