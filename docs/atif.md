# ATIF

## Overview

This page explains how `brain` uses `ATIF` internally and at the benchmark
boundary.

The important distinction is:

- `brain`'s operational runtime is still driven by its native event stream
- `ATIF` is the transcript and interchange format exported from that runtime
- the direct Harbor `brain` surface consumes the exported `ATIF` artifact
  rather than rebuilding trajectories in Python

`ATIF` is therefore a first-class export format in this repo, but it is not the
universal runtime event language.

## Three Layers

There are three separate layers to keep in mind:

1. operational runtime events
2. `ATIF` completion events on the runtime stream
3. persisted/exported `ATIF` trajectory state

### Operational runtime events

These are the events the runtime uses to describe what is happening while a
turn is executing:

- token deltas
- tool call lifecycle
- approvals
- retries
- progress
- `TurnDone`

Those events remain the main compatibility surface for transports, live UIs,
and loop/runtime behavior.

### `ATIF` completion events

When `AgentConfig.atif.emit_events` is enabled, `brain-core` also emits
`Event::Atif(...)` records on the same event stream.

These are completion-oriented transcript/export records:

- `TrajectoryStarted`
- `StepCompleted`
- `FinalMetrics`
- `TrajectoryCompleted`

They are additive. They do not replace token/tool/progress events.

### Persisted/exported trajectory

The runtime also maintains session-scoped `ATIF` trajectory state. That state is
append-only across turns and becomes the source for the final `trajectory.json`
artifact written by `brain exec`.

This separation exists to preserve transcript fidelity across turns. We do not
rebuild historical assistant steps from the current turn's config.

## Data Flow

The current direct `brain` flow is:

```text
provider + tools + loop
    -> native runtime events
    -> ATIF builder in brain-core
    -> Event::Atif(...)
    -> brain exec
    -> trajectory.json
    -> Harbor
```

The main responsibilities are:

- `brain-core`
  - runs the turn
  - persists new messages
  - appends the session `ATIF` trajectory
  - emits `ATIF` completion events
- `brain exec`
  - enables `ATIF` emission for direct runs
  - captures `TrajectoryCompleted`
  - writes `trajectory.json`, `events.jsonl`, and `run.json`
- Harbor direct `brain`
  - reads `run.json` for usage/cost metrics
  - consumes native Rust-emitted `trajectory.json`

## Turn Semantics

`ATIF` emission is part of turn completion.

When enabled, the runtime emits:

1. `TrajectoryStarted` when a session has no existing `ATIF` trajectory
2. zero or more `StepCompleted` events for newly completed steps in this turn
3. `FinalMetrics`
4. `TrajectoryCompleted`
5. `TurnDone`

`TurnDone` is intentionally still the terminal event for a successful turn.
Consumers that stop on `TurnDone` should still observe complete-turn semantics.

## Transcript Fidelity

The session trajectory is not regenerated from message replay plus current
config on every turn.

Instead, `brain-core` persists a session-level `ATIF` trajectory and appends
only the new steps produced by the current turn. That preserves historical
metadata such as:

- the original assistant `model_name`
- the original leading system prompt
- the original step ordering and IDs

This is why `TrajectoryStore` exists alongside message/session storage.

## Files And Ownership

The main code boundaries are:

- `crates/atif`
  - the reusable Rust schema and validation crate for `ATIF`
- `crates/brain-types/src/event.rs`
  - the runtime-facing `AtifEvent` stream surface
- `crates/brain-types/src/store.rs`
  - the session trajectory persistence surface
- `crates/brain-core/src/atif_events.rs`
  - append-only trajectory construction and `ATIF` completion events
- `crates/brain-core/src/brain.rs`
  - turn orchestration and event ordering
- `crates/brain-cli/src/exec_mode.rs`
  - direct-run sink that writes `trajectory.json`

## Non-Goals

The current design intentionally does not do the following:

- replace the native runtime event stream with `ATIF`
- rebuild historical transcript state from the latest turn config
- push Harbor-specific trajectory conversion into Python
- treat `ATIF` as the canonical in-memory runtime model

`ATIF` is the exported transcript/interchange view of a run. The native runtime
events and store/session model remain the operational source of truth.
