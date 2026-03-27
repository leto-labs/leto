# Proposal: add-opencode-runtime-compat-hardening-plan

## Why

`agent-server` now exposes a real OpenCode-compatible surface under
`/v1/compat/opencode`, and the pinned OpenCode web app can already connect to
it for basic local validation.

That is not the same thing as full behavioral compatibility.

The current compat layer appears to have three different maturity levels mixed
together:

1. route inventory that exists because the OpenAPI contract required it
2. partially real runtime behavior that is enough for basic connectivity and
   some prompt flows
3. placeholder or compat-local behavior that is still too weak for the full
   OpenCode UI lifecycle

The OpenCode `v1.3.2` source in `repocache/anomalyco/opencode` makes those
remaining expectations inspectable from source:

- the browser client wrapper and generated v2 SDK show the route and event
  inventory the web app actually uses
- the web app bootstrap and reducers show which routes and events the browser
  actually depends on
- the OpenCode runtime and server modules show additional semantics that are
  not obvious from the web app alone but still affect browser behavior

The deeper repocache pass also shows that several important surfaces were easy
to understate in a first web-app-only sweep:

- provider credential set or remove flows are separate from provider listing
  and provider auth metadata
- workspace reset and cleanup depends on instance disposal and worktree reset,
  not just on one workspace button
- OpenCode has both global events and instance-scoped event streams, and the
  browser depends on both global refresh events and directory-scoped state
- the experimental surface includes separate workspace versus worktree
  lifecycles that materially affect browser flows

We need an explicit planning change that scopes the remaining runtime work
before any further implementation happens. Without that, we risk treating “the
UI loads” as success while still missing live eventing, workspace lifecycle,
provider auth, permission/question flow integration, PTY behavior, or other
OpenCode-specific semantics.

## What Changes

This change is planning-only. It does not implement server behavior yet.

This change proposes to:

1. define the remaining behavior-level compatibility obligations for
   `agent-server` beyond simple route presence
2. scope those obligations by subsystem, including bootstrap, eventing,
   sessions, providers, permissions, files, workspaces, PTY, and MCP
   surfaces
3. define the implementation scope for the next compat-server-layer work in
   `agent-server`, rather than leaving later implementation to infer behavior
   from scattered source inspection
4. require later implementation work on the compat server layer to reconcile
   runtime behavior against the pinned OpenCode `v1.3.2` source in repocache,
   not only against the OpenAPI document
5. require that later implementation change the real compat server layer under
   `/v1/compat/opencode`, not paper over gaps in the OpenCode UI
6. require a deeper repocache pass before later implementation begins so this
   planning effort does not stop at the web app code paths already inspected
7. explicitly capture the deeper-pass discoveries around `config.providers`,
   `auth.set` or `auth.remove`, `instance.dispose`, instance-scoped
   `event.subscribe`, tool and resource inventory, and the split between
   workspace and worktree operations where they affect the browser client
8. add `agent-server` spec deltas that make behaviorally real OpenCode
   web UI compatibility a canonical requirement rather than an implicit goal

## Impact

- Modified capability: `agent-server`
- No runtime code changes are implemented by this change
- No browser automation is implemented by this change
- This change is the planning foundation for later implementation of the
  `agent-server` OpenCode compat layer, with the web UI as the primary target
