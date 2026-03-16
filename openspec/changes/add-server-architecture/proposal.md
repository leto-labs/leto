# Proposal: add-server-architecture

## Why

brain currently couples the engine and the UI in `Brain.run(&transport)` — one
loop, one transport, one process. This works for a PoC but breaks down when:

1. **Multiple clients**: a TUI, an IDE extension, and a web UI all need to
   talk to the same engine.

2. **TUI responsiveness**: the TUI should be a thin rendering layer, not the
   engine host.

3. **Session sharing**: start a conversation in the TUI, continue it in the IDE.

4. **Long-running operations**: MCP servers, model preloading, compaction
   should run independently of any client's lifecycle.

## Reference: OpenCode's Architecture

OpenCode solves this with a clean server/client split: the engine runs as a
server (HTTP REST + SSE), clients connect to it. The TUI can run in-process
(same process, Worker RPC) or connect over the network. Same API either way.

## What

### One server, clients connect

brain runs as a single server. Two ways to talk to it:

1. **In-process channels** — for the TUI (same process). Vanilla tokio
   broadcast + mpsc. Zero serialization overhead.

2. **HTTP REST + SSE** — for everything else. IDE extensions, web UIs,
   scripts, curl. One port, one protocol, universal.

No stdio JSON-RPC mode. No three separate adapters. One server.

### EventBus

tokio `broadcast` channel. All engine events flow through as `ServerEvent`
(brain `Event` + session context + timestamp). Subscribers receive events
regardless of whether they're in-process or HTTP/SSE clients.

### BrainApi trait

The internal contract. Implemented once by `BrainServer`. HTTP routes and
the in-process client both call the same methods:

```rust
trait BrainApi: Send + Sync {
    async fn create_session(&self) -> Result<Session>;
    async fn list_sessions(&self) -> Result<Vec<Session>>;
    async fn get_session(&self, id: Ulid) -> Result<Session>;
    async fn delete_session(&self, id: Ulid) -> Result<()>;
    async fn send_message(&self, session_id: Ulid, content: &str) -> Result<()>;
    async fn cancel_turn(&self, session_id: Ulid) -> Result<()>;
    fn subscribe(&self) -> broadcast::Receiver<ServerEvent>;
    async fn status(&self) -> Result<ServerStatus>;
}
```

`send_message` is fire-and-forget. Kicks off a turn async, returns Ok(()).
Events arrive via `subscribe()` / SSE.

### HTTP endpoints (axum)

| Endpoint | Method | Maps to |
|----------|--------|---------|
| `POST /sessions` | create | `create_session()` |
| `GET /sessions` | list | `list_sessions()` |
| `GET /sessions/:id` | get | `get_session(id)` |
| `DELETE /sessions/:id` | delete | `delete_session(id)` |
| `POST /sessions/:id/message` | send | `send_message(id, content)` |
| `POST /sessions/:id/cancel` | cancel | `cancel_turn(id)` |
| `GET /events` | SSE | `subscribe()` → stream ServerEvents |
| `GET /status` | status | `status()` |

axum's `Sse<impl Stream<Item = Result<Event>>>` maps directly to the
EventBus broadcast receiver. Heartbeat every 10 seconds.

### brain-cli integration

- `brain` (default) — starts server in-process + TUI. Also starts the HTTP
  endpoint so IDE extensions can connect to the same running instance.
- `brain serve` — starts server with HTTP endpoint only (headless, no TUI).

### ACP compatibility (optional, future)

If we want ACP support for editors that only speak ACP (subprocess + JSON-RPC
over stdio), we build a tiny bridge binary later:

```
IDE ←→ brain-acp-bridge (subprocess) ←→ BrainServer (HTTP)
```

The bridge is a ~100 line shim. Not a core server mode — just a thin HTTP
client that translates ACP JSON-RPC to REST calls. Separate concern.

### What happens to Transport?

The `Transport` trait becomes secondary. `Brain.run(&transport)` stays for
simple scripting and tests. Interactive clients use `BrainApi`. The TUI is
a BrainApi client, not a Transport.

## Change Dependencies

- **Requires**: `enrich-event-model` (ServerEvent wraps the enriched Event enum)
- **Required by**: `add-tui-transport` (TUI is a BrainApi client)
- **Required by**: `add-brain-cli` (CLI starts BrainServer, exposes HTTP endpoint)

## Impact

- **New crate**: `brain-server` (BrainServer, BrainApi, EventBus, HTTP routes)
- **New spec**: `brain-server`
- **Dependencies**: `axum` (HTTP/SSE), `tokio` (channels, already a dep)
- **Modifies**: `brain-engine` (Brain.run is secondary), `transport` (secondary)
- **Repocache**: `axum` and `agent-client-protocol` added for reference
- **No breaking changes**: Brain.turn() and Brain.run() unchanged
