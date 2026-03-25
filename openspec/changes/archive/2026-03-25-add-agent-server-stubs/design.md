# Design: add-agent-server-stubs

## Decision: `AgentCore` Is The Shared Consumer Boundary

The shared abstraction for local and remote consumers is `AgentCore`, not
`AgentRuntime`.

This keeps the public concept aligned with the v2 composition crate and makes
the locality split explicit:

- `AgentCoreNative` for embedded/in-process use
- `AgentCoreRemote` for future network-backed use

Construction remains implementation-specific. The store-taking builder belongs
to `AgentCoreNativeBuilder`, not to the shared trait.

## Decision: Shared Core Keeps Proxy Store Parity

`AgentCore` continues to expose `store()` so remote consumers can keep the same
high-level access pattern once `AgentCoreRemote` returns proxy store handles.

This follows the earlier remote-parity direction from the Brain-era planning
without reviving `BrainApi` as a separate client boundary.

## Decision: Canonical API Lives Under `/v1`

The canonical hosted API for `agent-server` is versioned from the start and
lives under `/v1`.

This avoids binding the rewrite forever to an unversioned root contract while
still making the new API the primary server surface.

## Decision: Turn Streaming Uses NDJSON, Runtime Bus Uses SSE

The canonical API uses:

- `POST /v1/sessions/{id}/turns` with chunked NDJSON for one request-scoped turn
- `GET /v1/events` with SSE for the live shared runtime bus

This fits the shape of `AgentCore::turn(...)` better than SSE and avoids the
state/protocol complexity of websocket multiplexing for the first remote-ready
server cut.

## Decision: OpenCode Compatibility Is Secondary And Version-Scoped

The compatibility surface is mounted only under `/v1/compat/opencode`.

This keeps the canonical API first-class while still making room for a clear
compatibility adapter with stable route names.

The first compatibility pass should:

- implement low-risk reads where the mapping is obvious
- expose live event streaming
- return explicit `501 not_implemented` responses for unresolved writes or
  semantics-heavy flows

Scope remains limited to the session-oriented app API rather than unrelated
OpenCode route families.
