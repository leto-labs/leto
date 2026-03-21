# Design: add-server-architecture

## Decision: One Runtime Interface, Multiple Implementations

The long-term boundary for clients is `BrainRuntime`, not `BrainApi`.

The intended split is:

- `BrainRuntimeNative` for embedded/in-process use
- a future remote implementation, `BrainRuntimeRemote`, for networked use

Both should implement the same `BrainRuntime` trait so client code can stay
stable across embedded and remote modes.

## Decision: Remote Runtime Returns Proxy Handles

`BrainRuntime` includes:

- `store()`
- provider/tool/loop registries
- turn execution
- runtime bus subscription
- model helper methods

That means a future remote runtime cannot just expose a few RPC helpers. It
must also return proxy implementations for:

- `Store`
- `ProjectStore`
- `SessionStore`
- `MessageStore`
- `CredentialStore`
- runtime registries

Those proxy objects will translate trait calls into the remote server
transport, but from the caller's perspective the interface should remain the
same as the embedded runtime.

## Decision: Remote Transport Must Preserve Both Turn And Bus Semantics

The future remote path must preserve two independent runtime surfaces:

1. request-scoped turn streaming from `turn(...)`
2. live runtime-bus subscription from `subscribe()`

That means the server transport must carry:

- per-turn event streams
- live bus events
- ordinary store and registry CRUD/lookup requests

The current design intentionally defers the concrete wire protocol. HTTP/SSE is
still a plausible choice, but the protocol should be chosen to satisfy
`BrainRuntime` parity rather than to preserve the older `BrainApi` shape.

## Decision: Current Clients Are Not Blocked By Server Work

`brain-cli` and `brain-acp` already use the embedded runtime path directly.

Therefore:

- `brain-server` is deferred from the active workspace for now
- remote CLI modes like `serve` and `attach` are future work
- server work should resume only once the remote-runtime parity story is clear
