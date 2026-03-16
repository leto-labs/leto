# Tasks: add-server-architecture

## Implementation Checklist

### Core: EventBus + BrainApi
- [x] Define `ServerEvent` struct (session_id, event, timestamp)
- [x] Implement `EventBus` using `tokio::sync::broadcast`
- [x] Define `BrainApi` trait with BoxFuture methods for object safety
- [x] Define `ServerStatus` struct (providers, tools, active_sessions, active_turns)
- [x] Define error types (`BrainError::TurnActive`, `BrainErrorCode::TurnActive`)

### Core: BrainServer
- [x] Implement `BrainServer` struct holding Brain + EventBus (via inner Arc pattern)
- [x] `BrainServer::new(brain)` — no single-project binding, supports multi-project
- [x] Track active turns per session (prevent concurrent turns on same session)

### Project management
- [x] `create_project`, `list_projects`, `get_project`, `update_project`, `delete_project`
- [x] `ProjectUpdate` derives `Serialize`/`Deserialize` for HTTP PATCH body

### Session management
- [x] `create_session(project_id)`, `list_sessions(project_id)` — scoped to project
- [x] `get_session(id)`, `delete_session(id)` — flat (session IDs globally unique)
- [x] `update_session(id, SessionUpdate)` — rename/update metadata
- [x] `SessionUpdate` derives `Serialize`/`Deserialize` for HTTP PATCH body

### Message history
- [x] `list_messages(session_id)` — delegates to `MessageStore::message_list`

### Turn management
- [x] `send_message(session_id, content)` — fire-and-forget, spawns async drain task
- [x] `send_message_stream(session_id, content)` — returns `EventStream`, also publishes to bus
- [x] `cancel_turn(session_id)` — triggers `CancellationToken` for active turn
- [x] `send_message` looks up session → project → `AgentConfig` dynamically

### Provider info
- [x] Add `ProviderInfo` and `ProviderModelInfo` structs to `brain-types`
- [x] Add `fn info(&self) -> ProviderInfo` to `Provider` trait
- [x] Implement `info()` on MockProvider, OpenAiProvider, OpenAiOAuthProvider, MistralRsProvider, LlamaCppProvider
- [x] `ProviderRouter::info()` aggregates all sub-provider models (dedup by ID)
- [x] `list_providers()` in BrainApi — returns provider info from `Brain.provider`
- [x] `OpenAiConfig` extended with `name` and `models` fields, populated from presets

### Credential management
- [x] `list_credentials()`, `get_credential(name)`, `save_credential(name, cred)`, `delete_credential(name)`
- [x] Delegates to existing `CredentialStore` trait

### In-process client (for TUI)
- [x] `BrainServer::client()` → returns `Arc<dyn BrainApi>` (clone of self)

### Events & status
- [x] `subscribe()` → returns `broadcast::Receiver<ServerEvent>`
- [x] `status()` → returns `ServerStatus` with `Vec<ProviderInfo>`, tool names, session count, active turn IDs

### HTTP server (axum)
- [x] Project CRUD: `POST/GET /projects`, `GET/PATCH/DELETE /projects/:id`
- [x] Session create/list under project: `POST/GET /projects/:pid/sessions`
- [x] Session get/update/delete (flat): `GET/PATCH/DELETE /sessions/:id`
- [x] Message history: `GET /sessions/:id/messages`
- [x] Send message (fire-and-forget): `POST /sessions/:id/message` → 202
- [x] Send message (streaming NDJSON): `POST /sessions/:id/message/stream` → 200 `application/x-ndjson`
- [x] Cancel turn: `POST /sessions/:id/cancel`
- [x] List providers: `GET /providers`
- [x] Credential CRUD: `GET /credentials`, `GET/PUT/DELETE /credentials/:name`
- [x] SSE stream: `GET /events` with 10s heartbeat
- [x] Status: `GET /status`
- [x] Error mapping: `BrainError` → appropriate HTTP status codes
- [x] `serve(server, addr)` convenience function + `build_router`

### brain-cli integration (deferred to add-brain-cli)
- [ ] `brain` (default): start server + HTTP endpoint + TUI
- [ ] `brain serve`: start server + HTTP endpoint only (headless)
- [ ] `--port` flag for HTTP port configuration

### Tests
- [x] Unit: EventBus subscribe, publish, multiple subscribers, dropped subscriber
- [x] Unit: BrainServer project CRUD
- [x] Unit: BrainServer create/get/delete/update session, sessions scoped to project
- [x] Unit: BrainServer send_message produces events via subscribe
- [x] Unit: BrainServer send_message_stream returns events and publishes to bus
- [x] Unit: concurrent turns on different sessions
- [x] Unit: reject concurrent turn on same session (TurnActive error)
- [x] Unit: cancel active turn, cancel no active turn is OK
- [x] Unit: active turn cleared after completion (can send again)
- [x] Unit: client() returns working BrainApi
- [x] Unit: status reports tools, sessions, and provider info
- [x] Unit: list_messages empty and after turn
- [x] Unit: update_session title
- [x] Unit: list_providers returns mock provider info
- [x] Unit: credential CRUD (save, get, list, delete)
- [x] Integration: HTTP project CRUD (create, list, get, delete)
- [x] Integration: HTTP session create/list/get/delete under projects
- [x] Integration: HTTP session update (PATCH)
- [x] Integration: HTTP list messages (empty + after turn)
- [x] Integration: HTTP send_message returns 202
- [x] Integration: HTTP send_message_stream returns NDJSON with token + TurnDone events
- [x] Integration: HTTP status returns 200 with provider info
- [x] Integration: HTTP providers returns 200
- [x] Integration: HTTP credential CRUD (list, get 404, put, get, list, delete, get 404)
- [x] Integration: invalid ULID returns 400 (session and project)
- [x] Integration: SSE stream receives events from send_message
