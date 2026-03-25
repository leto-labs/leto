# Proposal: add-opencode-compat-surface

## Why

Once the runtime, store, and hosted server path are ready, the fastest path to
a usable frontend is not a custom web app. The better path is to expose a
compatibility surface for the actively maintained `anomalyco/opencode`
client/server ecosystem and reuse its upstream UI with as few changes as
possible.

This should accelerate frontend delivery because:

1. `anomalyco/opencode` already has a desktop app, web app, SDK, and
   session-oriented HTTP API.
2. The upstream UI already supports configurable server URLs, auth, and remote
   connections, so Brain does not need a Brain-specific frontend just to get
   started.
3. Brain's project, session, provider, and credential concepts map well onto
   the upstream workspace, session, and provider/config flows.

However, it is too early to implement that compatibility layer today.

The repo's current direction is still:

- `BrainRuntime` as the canonical client-facing boundary
- future remote/server work built around `BrainRuntimeRemote` parity
- `brain-server` still out of the active workspace

There are multiple blockers that must be resolved before any compatibility
implementation should begin:

- the remote runtime contract is still deferred future work
- store cleanup and lifecycle semantics still need to be treated as hard
  prerequisites
- the hosted `BrainRuntimeNative` + `BrainServer` integration point is not yet
  settled
- the OpenCode endpoint and semantic mapping has not yet been specified

This change therefore exists to document the intended compatibility direction,
the required blocker gates, and the rule that frontend deltas must stay
minimal. It does **not** authorize implementation.

## What

This change adds planning-only OpenSpec deltas for:

1. `brain-server` requirements for an OpenCode-compatible secondary API.
2. `brain-core` requirements that keep `BrainRuntime` as the canonical
   boundary and require adapters to layer on top of it.
3. `brain-stores` requirements that treat store cleanup and event semantics as
   hard prerequisites for compatibility work.

The intended compatibility target is the active `anomalyco/opencode` codebase,
specifically the session-oriented app API used by its UI and SDK:

- project/workspace flows
- session lifecycle
- message history and message posting
- permission flows
- provider/config surfaces
- event streaming

This change explicitly does **not** promise parity for unrelated upstream
surfaces such as PTY, generic file routes, MCP routes, TUI-only routes, or
experimental routes.

## Frontend Direction

The frontend direction for later work is:

1. Keep Brain's canonical API and runtime model intact.
2. Add an OpenCode-compatible API surface as a secondary adapter.
3. Preserve the upstream UI's existing server configuration flows whenever
   possible.
4. Treat the `anomalyco/opencode` repo as a future submodule candidate, but do
   not add a submodule in this change.

Frontend changes are only acceptable for hard blockers such as:

- CORS / origin handling
- auth transport differences
- Tauri/webview networking constraints
- unavoidable platform integration seams

Frontend changes are not acceptable if they merely avoid building the adapter.

## Change Dependencies

- Depends on the remote-runtime/server direction documented by
  `add-server-architecture`
- Depends on store cleanup work that stabilizes project, session, message,
  credential, trajectory, and lifecycle-event semantics
- Depends on a future endpoint and semantic mapping matrix for
  `anomalyco/opencode`

## Impact

- Modified capability: `brain-server`
- Modified capability: `brain-core`
- Modified capability: `brain-stores`
- No code implementation authorized by this change
- Future implementation must remain blocked until the documented prerequisites
  are complete and approved
