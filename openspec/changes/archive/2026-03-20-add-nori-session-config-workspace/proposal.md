# Proposal: add-nori-session-config-workspace

## Why

`brain-acp` can already be validated against ACP clients, but the next product
question is no longer about protocol reach alone. It is about where richer
session controls should live.

The current research points to a clear split:

- ACP itself supports session-level `configOptions`
- clients are expected to prefer `configOptions` over older session modes when
  that surface is available
- Nori already provides the strongest terminal ACP client base in the current
  research set
- Nori does not yet appear to expose a generic ACP session-config UI in the
  inspected code path

That makes a client-side extension path cleaner than adding a `brain-acp`
bridge that invents custom behavior for one agent.

This change therefore establishes the fork workspace and implements the scoped
first ACP client extension directly in that workspace.

## What

This change will:

- add `git@github.com:leto-labs/nori-cli.git` as a git submodule at
  `submodules/nori-cli`
- create a development branch in that fork workspace named
  `feat/add-session-config`
- define a new OpenSpec capability for ACP client integration planning
- scope the first Nori-side ACP work to generic session config options only
- keep ACP session modes out of the initial branch
- add a dedicated `/session-config` UX surface rather than overloading
  `/model` or `/config`
- use the existing `codex-rs` source workflow (`just nori` / `cargo run`) as
  the primary development loop for the fork

## Impact

- Added capability: `brain-acp-integration`
- Repository workspace gains `submodules/nori-cli`
- The Nori fork workspace now contains the initial ACP session-config
  implementation on `feat/add-session-config`
- Local development stays aligned with upstream architecture by using the
  native Rust binary as the primary dev loop and leaving npm packaging as a
  later smoke-test concern
