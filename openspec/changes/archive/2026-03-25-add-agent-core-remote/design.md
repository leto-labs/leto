## Overview

`AgentCoreRemote` is the client-side implementation of the shared `AgentCore`
trait. It should not live in `agent-server`, because the server is the hosted
endpoint and the remote core is the consumer-side transport adapter.

This change adds a dedicated `agent-core-remote` crate that owns the canonical
`/v1` protocol types and, behind a `client` feature, the HTTP transport
implementation.

## Decisions

### Separate crate, not server-local implementation

- `agent-core` remains the abstraction crate.
- `agent-core-remote` owns the canonical hosted protocol and the HTTP client.
- `agent-server` depends on `agent-core-remote` without the client transport
  feature so the server and client share one canonical wire contract.

This avoids coupling the remote client to server internals while still keeping
testing simple.

### Native-first transport

The first implementation targets native Rust with `reqwest`,
`eventsource-stream`, and NDJSON decoding. Browser and WASM transport concerns
are deferred.

### Metadata is fetched at connect time

`AgentCore` contains synchronous metadata methods such as `provider_names()`,
`loop_names()`, `default_provider_name()`, `default_loop_name()`, and
`list_models()`.

Because a remote implementation cannot perform blocking network calls in those
methods, `AgentCoreRemote::connect(...)` fetches `/v1/status` and `/v1/models`
up front and caches that metadata locally.

### Canonical `/v1` must cover full store parity

The remote core must preserve `core.store()` access. That requires canonical
routes for:

- project root lookup and project resolution
- full project and session updates
- session listing across all projects
- message replacement and deletion
- credential CRUD and health updates
- trajectory listing and deletion

The remote store implementation uses those routes directly, while core-level
methods continue to use the higher-level canonical routes.

### Shared canonical protocol ownership

`agent-core-remote` owns the canonical `/v1` DTOs:

- health and status payloads
- request bodies
- typed provider and trajectory summaries
- structured error envelopes

Compatibility-layer DTOs remain in `agent-server`, because they are specific to
the OpenCode adapter rather than the canonical API.

## Trade-offs

### Why not a dedicated protocol crate?

A standalone protocol crate would reduce feature-gating complexity, but it adds
another top-level concept to an already split workspace. For now, keeping the
canonical protocol with `agent-core-remote` keeps the remote implementation and
its contract together.

### Why not make metadata methods async?

That would ripple through all current `AgentCore` consumers. Fetching and
caching the metadata at connect time keeps the existing shared trait intact and
works well for the current locality-transparent goal.
