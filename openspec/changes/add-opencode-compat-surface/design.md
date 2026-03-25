# Design: add-opencode-compat-surface

## Decision: Canonical Boundary Stays BrainRuntime

The OpenCode-compatible surface is a secondary API, not the source of truth.

Brain's internal architecture remains:

- `BrainRuntime` as the canonical client-facing boundary
- `BrainRuntimeNative` as the hosted embedded implementation
- a future `BrainRuntimeRemote` as the networked implementation

Any OpenCode-compatible HTTP layer must adapt **from** those canonical runtime
and store semantics. It must not redefine Brain's core abstractions around
OpenCode nouns or wire shapes.

## Decision: Target The Active anomalyco/opencode Consumer

The intended compatibility target is the actively maintained
`anomalyco/opencode` project rather than older unrelated `opencode` repos.

This target is justified because it already provides:

- a desktop app and web app
- a generated SDK
- a documented OpenAPI-based server surface
- configurable server URL selection
- auth-aware remote connections
- session-oriented event streaming

For planning purposes, the compatibility scope is the session-oriented app API
used by the upstream UI and SDK:

- project/workspace flows
- session lifecycle
- message history and posting
- permission flows
- provider/config flows
- event streaming

The following are intentionally out of scope for this change:

- PTY routes
- generic file routes
- MCP routes
- TUI-only routes
- experimental routes

## Decision: Frontend Changes Must Stay Minimal

The upstream UI already supports configurable and remote servers, so Brain
should meet the upstream contract rather than reshaping the UI around Brain's
current gaps.

The upstream UI behavior that drives this decision includes:

- persisted default server URLs in the web app entrypoint
- a server context that supports multiple HTTP and remote connection modes
- an explicit server-selection dialog with URL and auth inputs
- SDK creation from the currently selected server
- event streaming from the selected server rather than a fixed local backend

Therefore the future implementation must follow this rule:

- frontend deltas are allowed only for hard blockers such as CORS, auth
  transport, or platform networking constraints
- frontend deltas are not allowed if they serve mainly to avoid adapter work

Examples of disallowed changes:

- removing remote/custom server configuration
- inventing Brain-only frontend flows because an endpoint is missing
- hiding compatibility gaps behind temporary UI workarounds

## Decision: Compatibility Work Is Hard-Blocked

This change must be blocker-heavy and planning-only.

Implementation is forbidden until the following prerequisites are complete and
approved:

1. Remote runtime parity is specified well enough to support hosted
   `BrainRuntimeNative` plus a future `BrainRuntimeRemote`.
2. Store cleanup is complete for project, session, message, credential, and
   trajectory behavior, including lifecycle events needed for remote
   observation.
3. The future `brain-server` hosting model is defined around
   `BrainRuntimeNative`, not legacy `BrainApi`.
4. A written endpoint and semantic mapping matrix exists for the targeted
   OpenCode-compatible API surface.
5. Auth, approval, workspace-selection, and server mount decisions are
   specified.

## Decision: Future UI Adoption Is Separate

The `anomalyco/opencode` repo may later be adopted as a submodule or similar
upstream-tracking workflow, but that decision does not belong in this change.

This change only records that:

- the upstream repo is the intended frontend reference consumer
- future frontend adoption should try to preserve upstream UX and
  configurability
- submodule or fork mechanics are deferred to a later implementation change
