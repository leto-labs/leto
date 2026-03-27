# agent-server Delta Spec

## ADDED Requirements

### Requirement: OpenCode Compatibility Bootstrap Must Be Behaviorally Real

The OpenCode compatibility surface SHALL implement the bootstrap routes used by
the pinned OpenCode web UI consumer with usable runtime behavior, not only
contract shape.

This includes the routes and state loads needed for global bootstrap and
directory bootstrap, including path, config, project, provider, session
status, permission, question, VCS, command, MCP, LSP, config-provider
defaults, credential mutation routes, and related browser-facing global
surfaces.

#### Scenario: Browser bootstrap completes without empty replies
- **WHEN** the pinned OpenCode UI performs its initial global and directory
  bootstrap against `/v1/compat/opencode`
- **THEN** each required bootstrap route SHALL respond with usable data rather
  than an empty reply or panic
- **AND** the resulting state SHALL be sufficient for the UI to continue into
  normal project and session usage

### Requirement: OpenCode Compatibility Event Streams Use OpenCode-Native Semantics

The compatibility event streams SHALL emit OpenCode-native event shapes that
match the expectations of the pinned OpenCode reducers and caches.

Staying connected is necessary but not sufficient. The event payloads and event
names must be useful to the consumer.

This requirement covers the global event stream plus any shared disposal or
directory-scoped semantics the pinned OpenCode web UI depends on.

#### Scenario: Reducer-visible event types are emitted with usable payloads
- **WHEN** runtime activity changes project, session, message, todo,
  permission, question, PTY, or workspace state
- **THEN** the compat SSE stream SHALL emit the corresponding OpenCode-native
  event types with payloads the UI can apply directly
- **AND** the stream SHALL NOT rely on a generic compat-only catch-all event as
  the primary interoperability mechanism

### Requirement: OpenCode Compatibility Session Behavior Supports Real Interactive Use

The compatibility surface SHALL support the session behaviors exercised by the
pinned OpenCode app, including prompt flows, command flows, live status,
transcript updates, todo updates, diffs, summarize or compact actions,
revert or unrevert actions, share or unshare actions, and fork or child-session
behavior.

#### Scenario: Interactive session actions round-trip through compat behavior
- **WHEN** a caller performs OpenCode session actions such as create, prompt,
  abort, fork, summarize, revert, unrevert, share, or unshare
- **THEN** the compatibility layer SHALL provide behaviorally real results and
  follow-up events
- **AND** the UI SHALL be able to reconcile optimistic state with server truth

### Requirement: Missing OpenCode Behavior Must Be Implemented In The Compat Server Layer

Later OpenCode compatibility hardening MUST implement missing behavior in the
`agent-server` compat layer under `/v1/compat/opencode`.

Frontend or validation-consumer changes MAY be used only for narrow transport
or platform seams and SHALL NOT be used as the primary fix for missing compat
behavior.

#### Scenario: Missing compat behavior is discovered during OpenCode validation
- **WHEN** contributors discover that a required OpenCode workflow fails
  because the compat surface is incomplete or behaviorally incorrect
- **THEN** the required fix SHALL be scoped to the compat server layer by
  default
- **AND** the repository SHALL NOT treat a frontend workaround as a substitute
  for implementing the missing server behavior

### Requirement: OpenCode Compatibility Provider And Approval Flows Are Behaviorally Real

The compatibility surface SHALL support the provider, auth, permission, and
question flows that the pinned OpenCode app drives through its provider dialogs,
settings, and blocked-turn UX.

This includes provider inventory, provider auth metadata, API-key and
OAuth-style auth flows, config round-tripping, pending permission and question
lists, credential mutation through auth routes, and response or reply actions
that unblock the affected session.

#### Scenario: Provider and approval flows survive real UI usage
- **WHEN** a user connects providers, changes provider-related config, or
  responds to permission or question requests through the OpenCode UI
- **THEN** the compat routes SHALL provide coherent behavior and follow-up
  state updates
- **AND** SHALL NOT depend on placeholder responses disconnected from runtime
  state

#### Scenario: Deprecated and current permission reply routes remain compatible
- **WHEN** pinned OpenCode consumers use either the current permission reply
  route family or the deprecated session-scoped respond route family
- **THEN** the compatibility layer SHALL preserve behavior for both routes as
  required by the pinned release

### Requirement: OpenCode Compatibility Covers Workspace PTY MCP And Review Workflows

The compatibility surface SHALL provide deterministic behavior for the
workspace, worktree, file, review, PTY, MCP, and adjacent admin workflows used
by the pinned OpenCode app.

#### Scenario: Non-chat workflows remain usable through compat routes
- **WHEN** the OpenCode UI performs workspace, worktree, diff, file, PTY, MCP,
  or admin-driven interactions
- **THEN** the compatibility layer SHALL provide usable route behavior and
  follow-up state or events for those workflows
- **AND** SHALL NOT expose present-but-placeholder endpoints that break normal
  UI interaction

### Requirement: OpenCode Compatibility May Treat Non-Web Product Surfaces As Secondary

The compatibility surface SHALL prioritize behavior required by the pinned
OpenCode web UI.

Non-web, TUI-only, or broader product surfaces MAY be treated as secondary in
this hardening effort unless they reveal shared semantics needed for browser
compatibility.

#### Scenario: Deeper source review finds non-web surfaces
- **WHEN** contributors compare the compat checklist with pinned OpenCode
  runtime or admin modules and find additional non-web surfaces
- **THEN** those surfaces SHALL be added to the active hardening scope only if
  they affect browser-visible behavior or shared runtime semantics the web UI
  depends on

### Requirement: Compatibility Scope Must Be Reconciled Against Pinned OpenCode Source

Contributors MUST reconcile the OpenCode compatibility behavior checklist
against the pinned OpenCode source in repocache before later implementation
work claims compatibility hardening is complete.

That reconciliation SHALL use more than the OpenAPI document or the initially
inspected web-app code paths, but the primary acceptance target remains the
browser UI rather than full SDK parity.

#### Scenario: Deeper repocache pass extends the implementation checklist
- **WHEN** contributors review additional pinned OpenCode runtime and server
  modules during compat hardening
- **THEN** any newly discovered behavior-level expectations SHALL be folded into
  the approved implementation checklist before completion is claimed
