# Proposal: archive-brain-legacy-crates

## Why

The committed repo now centers on the `agent-*` and `provider-*` stack, but the
legacy `brain-*` crates still occupy workspace membership, active tooling
branches, and canonical OpenSpec capability slots.

That leaves the committed tree in an in-between state:

- legacy crates still look live even though they are no longer the product path
- mixed-use tooling still exposes legacy branches that require those crates
- canonical specs still describe archived legacy capabilities as current truth

We want to keep the legacy source available for reference without keeping it in
the committed repo.

## What Changes

- move committed `crates/brain-*` source into a local gitignored
  `archive/brain/` tree
- remove all committed workspace and tooling dependencies on those crates
- archive legacy-only examples, scripts, and Harbor helpers alongside the local
  legacy source tree
- retire canonical `brain-*` specs from `openspec/specs/` and keep them only as
  historical archive material

## Impact

- Modified capability: `repo-tooling`
- Removes `brain-*` crates from the committed workspace while preserving local
  reference copies
