# Proposal: adopt V4A apply_patch and add Attractor research resource

## Why

`brain-tools` currently implements `apply_patch` as a plain unified-diff parser.
That does not match the editing format used by Codex, OpenCode, Cline, and
other Codex-family clients and prompts, which use the V4A `*** Begin Patch`
envelope format. In practice this causes valid model-generated patches to fail
with parser errors even when ACP and tool plumbing are working correctly.

We also want `strongdm/attractor` available in `repocache` as a research source
for orchestrator and provider-aligned coding-agent loop design.

## What Changes

- Replace the `brain-tools` native `apply_patch` parser with a V4A parser and
  executor.
- Update the `apply_patch` tool description to document the V4A format
  explicitly.
- Remove unified-diff parsing support from `apply_patch`.
- Add `strongdm/attractor` to `repocache` with notes pointing to its loop and
  orchestration specs.

## Impact

- `apply_patch` becomes compatible with Codex/OpenCode-style patch output.
- Existing unified-diff-only patches stop being accepted by `apply_patch`.
- `repocache` gains a new research resource for orchestration and loop design.
