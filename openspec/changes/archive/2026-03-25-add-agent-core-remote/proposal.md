## Why

`AgentCore` is now the shared consumer-facing boundary, but there is still no
remote implementation that lets callers use the same interface against a hosted
`agent-server`.

The current canonical `/v1` server surface is also not broad enough to back the
full composed `Store` trait. That means a consumer using `core.store()` still
leaks locality: some operations only work in-process today.

We need a native-first HTTP client implementation of `AgentCore` that connects
to `agent-server` and preserves the same high-level access pattern for CLI,
ACP, web, and other clients.

## What Changes

- Add a new `agent-core-remote` crate.
- Make `agent-core-remote` own the canonical `/v1` wire DTOs used by both the
  remote client and `agent-server`.
- Implement `AgentCoreRemote` as an HTTP client over `agent-server`.
- Expand the canonical `/v1` surface so the full composed `Store` interface is
  reachable remotely, including project root lookup, full record updates,
  message replacement/deletion, credentials, and trajectory listing/deletion.
- Add live parity tests that run `AgentCoreRemote` against a real
  `agent-server`.

## Impact

- Affects `agent-core`, `agent-server`, and a new `agent-core-remote` crate.
- Broadens the canonical `/v1` API to cover the full store-backed remote
  contract.
- Keeps OpenCode compatibility server-local and secondary to the canonical API.
