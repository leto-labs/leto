# Design: add-opencode-runtime-compat-hardening-plan

## Decision: Keep This Change Planning-Only

This change exists only to author the implementation plan and canonical spec
requirements for deeper OpenCode runtime compatibility work in the
`agent-server` compat layer.

It does not authorize or perform:

- compat route implementation changes
- event translation changes
- browser automation changes
- OpenCode UI patches

Those actions belong to later implementation changes after this planning scope
has been approved, and those later changes should modify the compat server
layer itself rather than relying on frontend workarounds.

## Decision: Treat OpenCode Source As A Behavioral Oracle, Not Just The OpenAPI

The OpenAPI contract remains necessary, but it is not sufficient for real
OpenCode interoperability.

Later implementation work must reconcile against the pinned OpenCode
`v1.3.2` source in repocache, especially:

- `packages/sdk/js/src/v2/client.ts` for browser client wrapper behavior
- `packages/sdk/js/src/v2/gen/sdk.gen.ts` for browser-callable client
  namespaces and operation inventory
- `packages/sdk/js/src/v2/gen/types.gen.ts` for browser-visible event
  inventory
- `packages/app/src/context/global-sync/bootstrap.ts` for bootstrap behavior
- `packages/app/src/context/global-sync/event-reducer.ts` for reducer-visible
  event semantics
- `packages/app/src/pages/session.tsx`, `packages/app/src/pages/layout.tsx`,
  `packages/app/src/context/terminal.tsx`, and related dialogs/components for
  interaction-driven expectations
- `packages/opencode/src/server/routes/*` and adjacent runtime modules for
  behavior not obvious from the browser alone

The deeper pass already exposed several source-backed additions that belong in
the implementation scope:

- `config.providers` is distinct from `provider.list` and still matters where
  browser-visible provider or model defaults derive from it
- `auth.set` and `auth.remove` are first-class credential mutation routes used
  by the web app for API-key and custom-provider flows
- `instance.dispose` participates in provider reconnect and workspace reset
  flows
- the web app uses `global.event`, while deeper runtime inspection also shows
  instance-scoped event surfaces that influence shared semantics
- the experimental surface includes `tool.ids`, `tool.list`,
  `experimental.session.list`, and `experimental.resource.list`
- top-level server routes also expose `auth.set`, `auth.remove`, and
  `instance.dispose`, which are not confined to `server/routes/*.ts`
- `experimental.workspace.*` and `worktree.*` are separate lifecycle families
  with different payloads and semantics
- OpenCode currently carries both `permission.reply` and deprecated
  `permission.respond`, so compat hardening should preserve the routes the
  pinned consumers still use

This planning change is intentionally web-first. Non-web or TUI-only surfaces
are secondary and only matter here when they reveal shared runtime semantics
that the browser depends on.

This planning change must also require a deeper repocache pass before later
implementation begins. The initial inventory is broad, but it is not treated as
complete until the additional runtime/server modules have been reviewed for
surfaces the web UI may not exercise directly today.

## Decision: Model Compat As OpenCode-Native Semantics Over Shared Core Plus Compat-Local State

Later implementation of the `agent-server` compat server layer should follow
this model:

- use `AgentCore`, store, and runtime truth where the shared core already has a
  meaningful equivalent
- use compat-local server subsystems only where OpenCode has a concept that the
  shared core does not natively represent yet

This split is especially important for:

- session state and transcript behavior
- runtime status and eventing
- provider/model/credential state
- permission and question lifecycles
- project and workspace identity
- PTY and MCP surfaces

Compat-local state is acceptable where necessary, but it must be deterministic,
non-panicking, and coherent with the browser’s expectations.

## Decision: Event Translation Is A First-Class Requirement

The compat SSE stream cannot be considered complete if it only stays open and
emits a generic catch-all event wrapper.

The web app reducers and local caches expect concrete OpenCode-native events
such as:

- `server.connected`
- `server.instance.disposed`
- `project.updated`
- `session.created`
- `session.updated`
- `session.deleted`
- `session.status`
- `session.diff`
- `session.idle`
- `session.compacted`
- `session.error`
- `message.updated`
- `message.removed`
- `message.part.updated`
- `message.part.removed`
- `message.part.delta`
- `todo.updated`
- `command.executed`
- `file.edited`
- `file.watcher.updated`
- `vcs.branch.updated`
- `lsp.client.diagnostics`
- `lsp.updated`
- `permission.asked`
- `permission.replied`
- question lifecycle events
- PTY lifecycle events
- workspace or worktree lifecycle events

Later implementation must translate runtime activity into the event shapes that
the OpenCode UI actually consumes.

## Decision: Scope Runtime Hardening By User-Visible Subsystems

The planning checklist for later implementation of the compat server layer
should be organized by subsystem rather than by route file.

### Bootstrap and Global Surfaces

Later implementation must cover the routes and state loads the OpenCode app
uses during global and directory bootstrap, including:

- global health, dispose, and config
- project list/current/update and git initialization
- config get/update and provider-derived config views
- config provider defaults via `config.providers`
- provider list and auth metadata
- credential mutation via `auth.set` and `auth.remove`
- path and VCS state
- app agents
- command list
- MCP, LSP, and formatter status
- session status
- permission and question lists
- global event streams plus any shared directory or instance disposal semantics
  the browser depends on

The planning checklist must require these routes to return usable runtime data
without empty replies or panics.

### Session Lifecycle and Turn Behavior

Later implementation must scope behavior for:

- session list/create/get/update/delete
- parent and child session relationships
- fork flows
- prompt, promptAsync, command, and shell submission
- optimistic message reconciliation
- abort while busy
- message, part, and transcript mutation
- todo and diff state
- summarize or compact flows
- revert and unrevert flows
- share and unshare flows
- init or session bootstrap helpers

The OpenCode app already calls these flows directly or derives state from their
results and follow-up events.

### Provider, Auth, Config, and Model Behavior

Later implementation must scope:

- provider inventory and connected-state reporting
- provider auth metadata
- API-key auth flows
- OAuth authorize and callback flows
- credential add/remove behavior through `auth.set` and `auth.remove`
- custom provider config round-tripping
- disabled providers
- model inventory, defaults, and visibility dependencies

This also includes the real reconnect or reset behavior that OpenCode triggers
after credential changes, such as instance disposal and state reload.

This must be strong enough for the provider dialogs and settings flows in the
OpenCode app, not merely shaped like the contract.

### Projects, Workspaces, and Worktrees

Later implementation must scope:

- project resolution from directory or header-scoped requests
- project metadata updates
- git init from the session view
- workspace and worktree list/create/remove/reset flows
- root worktree versus derived workspace behavior
- ready and failed worktree or workspace state propagation
- session navigation across workspace directories

The deeper pass shows these are not interchangeable:

- `experimental.workspace.*` models control-plane workspaces
- `worktree.*` models git worktree sandbox lifecycle
- workspace reset in the web app also coordinates session listing, terminal
  cleanup, instance disposal, and session archiving

### Files, Search, Review, and Diff Behavior

Later implementation must scope:

- file list, read, and status
- file, text, and symbol search
- VCS state
- session diff payloads for the review panel
- review-related file opening behavior
- dirty-file state used by reset and delete UX

### Permissions, Questions, and Todo State

Later implementation must scope:

- pending permission and question request listings
- response, reply, reject, and auto-accept interactions
- session association for blocked turns
- todo updates during active work
- notification-friendly event transitions

This includes the current route-family split where pinned consumers may still
touch both `permission.reply` and deprecated `permission.respond`.

### PTY, MCP, and Admin Surfaces

Later implementation must scope:

- PTY list/create/get/update/remove/connect
- PTY lifecycle events and reconnect behavior
- MCP status/add/connect/disconnect and auth-adjacent flows
- command and tool inventories, including `tool.ids` and `tool.list`
- MCP resource inventory and other experimental admin listings
- formatter and LSP status plus any adjacent admin surfaces that directly
  affect the browser UI

## Decision: Require A Deeper Repocache Pass Before Later Implementation

This planning effort is intentionally exhaustive-in-intent, but it should not
pretend the first scan is perfect.

Before any later implementation change begins, contributors must perform a
deeper source pass through repocache to catch anything that may have been
missed in the current inventory, especially in browser-adjacent modules and in
shared runtime modules that can change browser behavior, including:

- `packages/opencode/src/server/routes/*`
- `packages/opencode/src/session/*`
- `packages/opencode/src/project/*`
- `packages/opencode/src/provider/*`
- `packages/opencode/src/permission/*`
- `packages/opencode/src/question/*`
- `packages/opencode/src/pty/*`
- `packages/opencode/src/mcp/*`
- `packages/opencode/src/installation/*`
- `packages/opencode/src/cli/cmd/tui/*`

The later implementation checklist must be updated if that deeper pass uncovers
additional behavior-level expectations.
