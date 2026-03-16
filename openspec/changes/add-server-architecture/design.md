# Design: add-server-architecture

## Decision: One Server, Clients Connect

```
        BrainServer (one process)
        ┌──────────────────────────────┐
        │  Brain (engine)              │
        │  EventBus (broadcast)        │
        │  BrainApi (internal trait)   │
        │                              │
        │  HTTP/SSE endpoint (axum)    │◄──── IDE extension (HTTP client)
        │                              │◄──── Web UI (HTTP client)
        │                              │◄──── ACP bridge (thin proxy, optional)
        │  In-process channels         │◄──── TUI (same process)
        └──────────────────────────────┘
```

brain runs as a single server. Two ways to talk to it:

1. **In-process channels** — for the TUI, which lives in the same process.
   Vanilla tokio, zero overhead.

2. **HTTP REST + SSE** — for everything else. One port, one protocol.
   IDE extensions, web UIs, scripts — all connect the same way.

That's it. No stdio JSON-RPC mode. No three separate adapters.

### ACP compatibility (optional, later)

If we want ACP compatibility for editors that only speak ACP (spawn
subprocess + JSON-RPC over stdio), we can build a tiny bridge binary:

```
IDE ←→ brain-acp-bridge (subprocess) ←→ BrainServer (HTTP)
```

The bridge is a ~100 line shim that translates ACP JSON-RPC to HTTP calls
against the running server. It's not a server mode — it's a thin client.

### Why HTTP + SSE?

- **SSE for events**: server pushes events (tokens, tool calls) to all
  connected clients. Native axum support.
- **REST for commands**: create session, send message, cancel. Standard
  HTTP semantics.
- **Universal**: any language, any platform can be an HTTP client. No SDK
  required. `curl` works for debugging.
- **NDJSON streaming**: for clients that want a request-response model,
  `POST /sessions/:id/message/stream` returns newline-delimited JSON.

## Decision: Multi-Project, RESTful Nesting

`BrainServer` supports multiple projects simultaneously. This differs from
OpenCode's approach (implicit workspace context via headers).

- **Projects** are top-level resources at `/projects`.
- **Sessions** are created under projects (`POST /projects/:pid/sessions`)
  and listed under projects (`GET /projects/:pid/sessions`).
- **Session operations** are flat (`GET/PATCH/DELETE /sessions/:id`,
  `POST /sessions/:id/message`) since session IDs are globally unique.

This is stateless and multi-client safe. Two TUI clients or an IDE + TUI
can work on different projects against the same server without confusion.

## Decision: Dual Send Endpoints

Two ways to send a message, for different client patterns:

1. **Fire-and-forget** (`POST /sessions/:id/message` → 202): starts the
   turn, returns immediately. Events flow to SSE subscribers. Best for
   multi-client setups where all clients observe via `GET /events`.

2. **Streaming** (`POST /sessions/:id/message/stream` → 200 NDJSON):
   starts the turn, returns an `EventStream` as NDJSON in the response
   body. Events also publish to the EventBus (SSE subscribers still see
   them). Best for simple request-response clients.

### BrainApi trait

The internal contract. Implemented once (by BrainServer). The HTTP routes
are thin wrappers that call BrainApi methods.

```rust
trait BrainApi: Send + Sync {
    // Project management
    fn create_project(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>>;
    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn update_project(&self, id: ProjectId, update: ProjectUpdate) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>>;

    // Session management (create/list scoped to project)
    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn list_sessions(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Vec<Session>, BrainError>>;
    fn get_session(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn update_session(&self, id: Ulid, update: SessionUpdate) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_session(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;

    // Message history
    fn list_messages(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>>;

    // Turn management
    fn send_message(&self, session_id: Ulid, content: &str) -> BoxFuture<'_, Result<(), BrainError>>;
    fn send_message_stream(&self, session_id: Ulid, content: &str) -> BoxFuture<'_, Result<EventStream, BrainError>>;
    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;

    // Provider & credentials
    fn list_providers(&self) -> BoxFuture<'_, Result<Vec<ProviderInfo>, BrainError>>;
    fn list_credentials(&self) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>>;
    fn get_credentials(&self, provider_name: &str) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>>;
    fn save_credential(&self, provider_name: &str, entry: CredentialEntry) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_credential(&self, provider_name: &str, credential_id: &str) -> BoxFuture<'_, Result<(), BrainError>>;

    // Events & status
    fn subscribe(&self) -> broadcast::Receiver<ServerEvent>;
    fn status(&self) -> BoxFuture<'_, Result<ServerStatus, BrainError>>;
}
```

`send_message` is fire-and-forget. Starts a turn async, returns Ok(()).
Events arrive via `subscribe()` (in-process broadcast) or SSE (HTTP).

`send_message_stream` starts a turn, returns an `EventStream` directly.
Events are also published to the bus (dual delivery).

### brain-cli modes

- `brain` remains to be defined as the dual-mode runtime combining interactive and HTTP behavior (not yet implemented in this worktree).
- `brain serve` currently starts server-only behavior (no TUI).

### Dependency summary

| Component | Crate | Required? |
|-----------|-------|-----------|
| EventBus | `tokio` (broadcast) | always |
| HTTP/SSE | `axum` | yes (core server feature) |
| ACP bridge | `agent-client-protocol` | optional, separate binary |
