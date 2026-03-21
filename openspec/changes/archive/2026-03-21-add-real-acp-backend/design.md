# Design: add-real-acp-backend

## Decision: Mirror The Real And Mock ACP Backends

The crate should expose two internal implementations with the same top-level
shape:

- `src/backend/*` for the real ACP backend
- `src/mock/*` for the mock ACP backend

This makes it possible to compare the same ACP responsibilities across both
implementations:

- connection bootstrap
- request handling
- session bookkeeping
- history replay
- event mapping
- capability advertisement

The mirrored structure is for readability first. Shared helpers should only be
introduced when they remain clearer than duplicated code.

## Decision: brain-acp Stays The ACP Boundary

ACP SDK request, response, notification, and update types should stay confined
to `brain-acp`.

`brain-core` and `brain-types` should not become ACP-shaped. They should remain
transport-agnostic runtime abstractions that can support ACP, CLI, HTTP, and
future transports equally well.

The adapter boundary is:

- ACP SDK types at the edge inside `brain-acp`
- `brain` domain types internally
- explicit mapping between the two in `event_mapper` and `history_replay`

## Decision: First Real Slice Uses BrainRuntime Directly

The real ACP backend should depend directly on `Arc<dyn BrainRuntime>`, not on
`brain-server` and not on ACP-local adapter state.

That keeps the ACP path close to the runtime already used by CLI and avoids
pulling HTTP/server concerns into the ACP integration surface.

## Decision: Project Identity Comes From Session cwd

ACP session creation and loading should resolve a `brain` project from the ACP
working directory.

This requires a store-level project lookup by normalized root so both the real
ACP backend and other runtime consumers can resolve the same project identity.

## Decision: First Real Slice Is Narrow

The first runtime-backed ACP backend should support:

- initialize
- new session
- load session
- list sessions
- prompt
- cancel
- session config options backed by runtime state

It should not yet advertise or implement:

- client-owned ACP requests
- session modes
- session model switching
- fork/resume/close flows

## Decision: Stored Tool Replay Is Deferred

The current `brain` store persists user/assistant/tool messages, but it does
not preserve enough structured tool lifecycle information to reconstruct
ACP-native stored tool history faithfully.

For `session/load`, the first real backend should replay:

- system messages
- user messages
- assistant messages

It should skip stored tool messages until the runtime model can support richer
replay safely.
