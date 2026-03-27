# Proposal: add-opencode-ui-validation-matrix-plan

## Why

We now have three things that make full UI validation practical:

1. a pinned OpenCode UI submodule under `submodules/opencode`
2. a real OpenCode-compatible `agent-server` surface under
   `/v1/compat/opencode`
3. working local browser automation for development and debugging

What we do not have yet is a maintained, exhaustive definition of what “OpenCode
works against our server” actually means.

Without that matrix, validation is too easy to underscope:

- the UI can connect, but eventing can still be wrong
- prompt flows can work, but provider setup or worktree creation can still
  break
- some routes can exist, but the review panel, PTY tabs, MCP dialog, or
  blocked-turn UX can still fail in practice

The pinned OpenCode `v1.3.2` repocache gives us enough source to define that
matrix from actual app behavior and the browser-facing v2 SDK, rather than
from guesswork.

The deeper repocache pass also surfaced important interaction details that were
easy to flatten in the first draft:

- provider connect and disconnect flows depend on `auth.set`, `auth.remove`,
  config updates, and sometimes instance disposal
- workspace reset is a coordinated flow involving session listing, terminal
  cleanup, instance disposal, worktree reset, and archived-session UX
- some shared runtime surfaces are broader than the obvious web calls, but they
  only belong in this matrix when they affect browser-visible behavior

We need a planning-only OpenSpec change that defines the full validation
surface and future automation expectations before writing any broader browser
test harness.

## What Changes

This change is planning-only. It does not implement browser automation or UI
tests yet.

This change proposes to:

1. define the exhaustive OpenCode UI interaction matrix that later validation
   and automation must cover
2. map each interaction family to the underlying SDK surfaces, compat routes,
   expected events, and visible UI outcomes
3. make that matrix the source-driven checklist for later implementation of the
   `agent-server` compat layer, not just for later testing
4. separate a later “must-pass smoke set” from the full regression matrix
5. require that future validation stay aligned with the pinned OpenCode release
   and pinned submodule
6. explicitly capture the deeper-pass discoveries around credential mutation,
   instance disposal, and workspace reset so they do not fall through the
   cracks
7. require a deeper repocache pass before later implementation begins so the
   matrix is not limited to only the app code paths inspected in the first pass

## Impact

- Modified capability: `repo-tooling`
- No browser test code is implemented by this change
- No automation harness is implemented by this change
- This change defines the source-driven checklist for later compat-layer
  implementation and validation work, with pinned web UI compatibility as the
  primary target
