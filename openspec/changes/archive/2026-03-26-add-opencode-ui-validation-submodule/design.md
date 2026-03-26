# Design: add-opencode-ui-validation-submodule

## Decision: Use A Submodule Instead Of A Loose Clone

`repocache` is the right place for source inspection, but it is not the right
place for a runnable validation target that may need small tracked edits.

A committed submodule is better because it:

- gives the repo a visible, reviewable version pin
- allows small local edits without losing sight of what diverged from upstream
- makes onboarding and reproduction straightforward for anyone testing the
  compatibility surface

## Decision: Use The Synced Fork, But Pin To The Release Commit

The submodule remote should point at `leto-labs/opencode`, but the checkout
must remain pinned to the exact commit that matches the current compatibility
target instead of following `dev`.

Today that means:

- compatibility target: upstream OpenCode `v1.3.2`
- canonical commit: `0dcdf5f529dced23d8452c9aa5f166abb24d8f7c`
- fork status: that commit is reachable from `leto-labs/opencode` `dev`

This preserves two useful properties:

1. we can carry tiny local fixes in the fork if needed
2. we do not silently drift away from the compat contract currently pinned by
   the implemented compat work and `openapi/opencode.json`

There is one current source-of-truth mismatch to correct during implementation:

- the archived compat implementation work is pinned to `v1.3.2`
- the local repocache checkout is at `v1.3.2`
- `repocache/repocache.json` still lists `main`

The implementation should resolve that mismatch instead of copying it into the
submodule workflow.

## Decision: Treat UI Edits As Exceptions, Not The Plan

The OpenCode app already supports:

- configurable server URLs
- persisted default server selection
- optional username/password auth for HTTP servers
- health checks against the configured server

The current `agent-server` spec already requires:

- CORS preflight support for browser-facing clients
- OpenCode-compatible SSE behavior with connected and heartbeat events

So the default assumption should be that the upstream UI runs unchanged against
our compat surface.

Local UI edits are allowed only when they remove a true blocker such as:

- auth transport mismatches we cannot reasonably solve server-side
- browser or Tauri networking constraints
- narrowly scoped CORS or origin behavior that cannot be represented through
  the existing server configuration

## Decision: Start With A Small Validation Workflow

The first implementation should stay narrow:

1. add the submodule
2. document the exact pinned version
3. document how to run the OpenCode web UI against local `agent-server`
4. smoke-test the core flows that the compat surface is meant to support

Only after that should we decide whether additional automation or desktop/Tauri
coverage is worth the maintenance cost.
