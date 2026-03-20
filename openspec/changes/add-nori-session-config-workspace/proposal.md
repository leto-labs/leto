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

The repo therefore needs two things before any Nori implementation starts:

- a checked-out fork workspace that can be used for local development and
  cross-testing
- an approved scope that keeps the first client change small and protocol-native

## What

This change will:

- add `git@github.com:leto-labs/nori-cli.git` as a git submodule at
  `submodules/nori-cli`
- create a development branch in that fork workspace named
  `feat/add-session-config`
- define a new OpenSpec capability for ACP client integration planning
- scope the first Nori-side ACP work to generic session config options only
- defer ACP session modes from the initial branch
- recommend a dedicated session-settings UX surface rather than overloading
  `/model` or `/config`

## Impact

- Added capability: `brain-acp-integration`
- Repository workspace gains `submodules/nori-cli`
- No runtime code changes in `brain-acp` or the Nori fork are included in this
  change
