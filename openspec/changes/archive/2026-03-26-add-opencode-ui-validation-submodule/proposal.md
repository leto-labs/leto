# Proposal: add-opencode-ui-validation-submodule

## Why

`agent-server` now has a real OpenCode compatibility surface and the canonical
`agent-server` spec explicitly requires browser-facing compatibility behavior
such as CORS preflight support and long-lived SSE streams.

The next useful step is to validate that surface against the actual OpenCode UI
instead of continuing to reason only from the exported OpenAPI contract and the
local `repocache` clone.

The existing planning change `add-opencode-compat-surface` intentionally
deferred submodule mechanics. That made sense before the compat layer existed,
but it is now stale relative to the implemented `agent-server` work.

Using a pinned submodule for the OpenCode UI is the most practical validation
setup because it:

1. gives the repo a runnable, versioned reference consumer for real browser and
   desktop-style testing
2. makes any unavoidable local UI patches explicit in Git instead of burying
   them in an out-of-band clone
3. keeps the validation target aligned with the already-pinned compatibility
   reference in `repocache`

## What

This change proposes to:

1. add the OpenCode repo as a Git submodule under `submodules/opencode`
2. use the synced `leto-labs/opencode` fork as the submodule remote
3. pin the submodule to the exact commit corresponding to the OpenCode
   compatibility release already used by the implemented compat work and the
   local repocache checkout, namely upstream `v1.3.2`, rather than tracking a
   floating branch like `dev`
4. document a lightweight validation workflow for running the pinned OpenCode
   UI against local `agent-server`
5. allow only very small, blocker-driven fork edits for issues such as auth,
   CORS, or transport seams, and require those edits to stay explicit and
   minimal
6. sync the `repocache` manifest pin to the same approved compatibility release
   if it is still left floating

## Impact

- Modified capability: `repo-tooling`
- No immediate runtime API changes required
- Future implementation should prefer web-first validation before taking on
  desktop/Tauri-specific seams
