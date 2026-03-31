# Proposal: restore-agent-benchmark-workflows

## Why

Archiving the legacy `brain-*` crates removed committed benchmark workflows
without replacing them on the live `agent-*` stack.

The repo lost:

- the repo-local direct Harbor agent surface
- the repo-local Harbor ACP surface
- the `acpx` smoke harness and launcher path

Current docs and specs already expect those workflows to exist on the live
stack, so the committed repo needs migrated replacements rather than historical
deletion only.

## What Changes

This change restores the repo-owned benchmark and ACP validation workflows on
the current architecture:

1. add committed repo-local Harbor `agent` and `agent-acp` surfaces
2. restore the `acpx` smoke harness through an explicit `agent-acp-mock` path
3. document and require direct `agent-acp` binary identities for repo-owned
   validation and Harbor usage

## Impact

- Modified capability: `agent-benchmarks`
- Modified capability: `agent-acp`
