# Proposal: reorganize-provider-and-tool-ecosystem-folders

## Why

The workspace currently stores standalone provider crates and standalone tool
crates as flat siblings under `crates/`. That works technically, but it blurs
two ecosystems that are intended to stay internally coherent and eventually may
be extracted more easily.

Grouping them under dedicated ecosystem folders makes the repository easier to
scan without changing crate identities or public Rust APIs.

## What Changes

- move provider crates under `crates/agent-provider/`
- move tool crates under `crates/agent-tool/`
- keep existing Cargo package names unchanged
- update workspace paths, tooling, docs, and specs that reference the old flat
  directories

## Impact

- no public crate names or Rust import paths change
- repo tooling and docs that reference concrete filesystem paths must be updated
- this is a structural refactor only; provider and tool behavior stays the same
