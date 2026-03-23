# Proposal

## Why

`brain` now has a Harbor-friendly direct execution path via `brain-cli exec`
and native ATIF export, but the current loop implementations are still tuned
around native provider tool calling. That architecture does not match the
high-performing Terminal Bench 2.0 loop family represented by Harbor's
`terminus_2.py`, which uses:

- structured text planning instead of native tool calls
- a persistent interactive terminal session
- bounded command wait windows with incremental terminal-state feedback
- parse-and-reprompt recovery
- explicit task-completion confirmation
- context summarization handoff when context pressure rises

To target benchmark parity with Harbor's Terminus-2 strategy while preserving
`brain`'s composability goals, we need a first-class Rust loop implementation
that fits the existing runtime abstractions instead of patching `RobustLoop`.

## What Changes

- Add a new `Terminus2Loop` in `brain-loops`, implemented in
  `crates/brain-loops/src/terminus2.rs`.
- Add a native stateful terminal-session tool in `brain-tools` so loops can
  execute Harbor-style keystroke batches against a persistent shell without
  teaching `brain-core` about a special loop.
- Extend shared message/runtime primitives only where needed to support
  Terminus-2 behavior across loops, especially assistant reasoning-content
  carry-forward and loop-visible inference failure classification.
- Register the new loop in native runtime bootstrap so `brain exec --loop
  terminus2` and other runtime surfaces can select it.
- Preserve Harbor-compatible ATIF emission for the new loop, including
  task-completion confirmation and summarization handoff steps.

## Impact

- Affected specs:
  - `brain-loops`
  - `brain-tools`
  - `brain-types`
  - `brain-cli`
  - `brain-core`
- Affected crates:
  - `brain-loops`
  - `brain-tools`
  - `brain-types`
  - `brain-core`
  - `brain-cli`
  - `brain-acp`
