# Add Direct Harbor Brain Agent And Native ATIF Export

## Why

The current Harbor validation path for `brain` goes through ACP. That is the
right path for protocol validation, but it mixes ACP overhead and ACP failure
shape into loop iteration.

We need a second Harbor surface that runs the local `brain` binary directly so
we can benchmark `brain-core` loop behavior without ACP in the middle.

That direct path also needs to own its benchmark artifacts cleanly. In
practice, that required making ATIF a first-class Rust capability rather than a
Harbor-side reconstruction step.

## What Changes

- add a repo-local direct Harbor `brain` agent under `tools/harbor/agents`
- add a non-interactive `brain exec` subcommand for one-shot Harbor runs
- make the direct Harbor `brain` path use explicit API-key auth rather than
  host `~/.brain` state
- make direct `brain` default to the Responses API when the selected
  OpenAI-compatible provider preset declares support
- add a reusable Rust `atif` crate aligned to Harbor ATIF v1.6
- emit native ATIF from Rust, including runtime completion events and persisted
  trajectory state
- document and wire the new Harbor runner surface

## Impact

- loop iteration can be benchmarked directly through Harbor
- ACP remains available as a separate comparison surface
- direct Harbor runs become host-independent apart from explicit API-key input
- ATIF becomes a reusable Rust export capability rather than a Harbor-only
  adapter concern
