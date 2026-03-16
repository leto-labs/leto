# brain-server Specification

## Purpose
HTTP and event infrastructure for the brain runtime, exposing multi-project/session control and turn events through a server API.

## Requirements

### Requirement: ServerEvent
The system SHALL define a `ServerEvent` struct that wraps a brain `Event` with session context:
- `session_id: Ulid` — which session the event belongs to
- `event: Event` — the brain event
- `timestamp: DateTime<Utc>` — when the event occurred

#### Scenario: Event with session context
- **WHEN** a turn emits a `Token { delta }` event for session X
- **THEN** the ServerEvent SHALL carry `session_id = X` and `event = Token { delta }`

### Requirement: EventBus
The system SHALL provide an `EventBus` struct backed by `tokio::sync::broadcast` that distributes `ServerEvent` instances to all subscribers.

- `subscribe() -> broadcast::Receiver<ServerEvent>` — create a new subscription
- `publish(event: ServerEvent)` — send to all subscribers

#### Scenario: Multiple subscribers
- **WHEN** two clients subscribe to the EventBus
- **AND** a ServerEvent is published
- **THEN** both subscribers SHALL receive the event

#### Scenario: Subscriber disconnect
- **WHEN** a subscriber drops their receiver
- **THEN** the EventBus SHALL continue functioning for remaining subscribers

#### Scenario: Slow subscriber
- **WHEN** a subscriber falls behind (lagged receiver)
- **THEN** the EventBus SHALL skip events for that subscriber
- **AND** log a warning

### Requirement: BrainApi Trait
The system SHALL define a `BrainApi` trait that represents the client-facing API of the engine. All async methods use `BoxFuture` for object safety.

```rust
trait BrainApi: Send + Sync {
    // Project management
    fn create_project(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>>;
    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn update_project(&self, id: ProjectId, update: ProjectUpdate) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>>;

    // Session management
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

#### Scenario: Trait is object-safe
- **WHEN** a caller has an `Arc<dyn BrainApi>`
- **THEN** it SHALL be able to call all methods without knowing the concrete implementation

### Requirement: BrainServer
The system SHALL provide a `BrainServer` struct that wraps a `Brain` and an `EventBus`. It implements `BrainApi`, manages turn lifecycle, and publishes events. `BrainServer` supports multiple projects — it does not bind to a single project at construction.

#### Scenario: Server construction
- **WHEN** `BrainServer::new(brain)` is called
- **THEN** it SHALL create a server with an EventBus ready to accept clients

#### Scenario: Multi-project support
- **WHEN** projects are created via `create_project()`
- **THEN** sessions SHALL be scoped to their project via `create_session(project_id)`
- **AND** `list_sessions(project_id)` SHALL return only sessions for that project

#### Scenario: Send message triggers turn
- **WHEN** `send_message(session_id, content)` is called
- **THEN** the server SHALL look up the session's project to obtain `AgentConfig`
- **AND** spawn an async task that calls `Brain.turn()`
- **AND** each event SHALL be published to the EventBus as a ServerEvent

#### Scenario: Send message stream
- **WHEN** `send_message_stream(session_id, content)` is called
- **THEN** the server SHALL start a turn and return an `EventStream` to the caller
- **AND** each event SHALL also be published to the EventBus (dual delivery)
- **AND** the active turn SHALL be cleaned up when the stream completes

#### Scenario: Events arrive in order
- **WHEN** a turn produces events Token("a"), Token("b"), TurnDone
- **THEN** subscribers SHALL receive them in that order

#### Scenario: Cancel turn
- **WHEN** `cancel_turn(session_id)` is called during an active turn
- **THEN** the CancellationToken SHALL be triggered
- **AND** the turn SHALL emit an Error event with Cancelled code

#### Scenario: No concurrent turns per session
- **WHEN** `send_message()` is called for a session that already has an active turn
- **THEN** it SHALL return `BrainError::TurnActive`

#### Scenario: Concurrent turns on different sessions
- **WHEN** turns are started on session A and session B
- **THEN** both SHALL run concurrently

### Requirement: In-Process Client
The system SHALL provide `BrainServer::client()` which returns an `Arc<dyn BrainApi>` for in-process use. This is the default for the TUI.

#### Scenario: In-process client
- **WHEN** `server.client()` is called
- **THEN** it SHALL return an `Arc<dyn BrainApi>` backed by a clone of the server

#### Scenario: No serialization
- **WHEN** the in-process client communicates with the server
- **THEN** it SHALL use typed Rust channels with no serialization overhead

### Requirement: Provider Info
The `Provider` trait SHALL include an `info()` method that returns `ProviderInfo`:

```rust
pub struct ProviderInfo {
    pub name: String,
    pub default_model: Option<String>,
    pub models: Vec<ProviderModelInfo>,
}

pub struct ProviderModelInfo {
    pub id: String,
    pub name: String,
    pub reasoning: bool,
    pub tool_call: bool,
}
```

#### Scenario: Provider reports capabilities
- **WHEN** `provider.info()` is called
- **THEN** it SHALL return the provider name, default model, and available models

#### Scenario: Router aggregates providers
- **WHEN** `ProviderRouter::info()` is called
- **THEN** it SHALL aggregate models from all sub-providers, deduplicating by model ID

### Requirement: Credential Management
The system SHALL expose credential CRUD via the `BrainApi` trait, delegating to the existing `CredentialStore`.

#### Scenario: Save and retrieve API key entries
- **WHEN** `save_credential("openai", CredentialEntry::api_key("openai-key", "sk-..."))` is called
- **THEN** `get_credentials("openai")` SHALL return the saved entry

#### Scenario: Delete credential
- **WHEN** `delete_credential("openai", "openai-key")` is called
- **THEN** `get_credentials("openai")` SHALL not contain an entry with id `openai-key`

### Requirement: HTTP Server
The system SHALL provide an HTTP server using axum that exposes BrainApi as REST endpoints and engine events as an SSE stream.

| Endpoint | Method | Maps to | Response |
|----------|--------|---------|----------|
| `POST /projects` | create | `create_project(body)` | 201 |
| `GET /projects` | list | `list_projects()` | 200 |
| `GET /projects/:id` | get | `get_project(id)` | 200 |
| `PATCH /projects/:id` | update | `update_project(id, body)` | 204 |
| `DELETE /projects/:id` | delete | `delete_project(id)` | 204 |
| `POST /projects/:pid/sessions` | create | `create_session(pid)` | 201 |
| `GET /projects/:pid/sessions` | list | `list_sessions(pid)` | 200 |
| `GET /sessions/:id` | get | `get_session(id)` | 200 |
| `PATCH /sessions/:id` | update | `update_session(id, body)` | 204 |
| `DELETE /sessions/:id` | delete | `delete_session(id)` | 204 |
| `GET /sessions/:id/messages` | list | `list_messages(id)` | 200 |
| `POST /sessions/:id/message` | send | `send_message(id, content)` | 202 |
| `POST /sessions/:id/message/stream` | stream | `send_message_stream(id, content)` | 200 NDJSON |
| `POST /sessions/:id/cancel` | cancel | `cancel_turn(id)` | 200 |
| `GET /providers` | list | `list_providers()` | 200 |
| `GET /credentials` | list | `list_credentials()` | 200 |
| `GET /credentials/:name` | get | `get_credentials(name)` | 200 |
| `PUT /credentials/:name` | save | `save_credential(name, body)` | 204 |
| `DELETE /credentials/:name/:credential_id` | delete | `delete_credential(name, credential_id)` | 204 |
| `GET /events` | SSE | `subscribe()` → stream | 200 SSE |
| `GET /status` | status | `status()` | 200 |

Session create/list are nested under projects (`/projects/:pid/sessions`). Session get/update/delete/message/cancel are flat (`/sessions/:id`) since session IDs are globally unique.

#### Scenario: HTTP server startup
- **WHEN** the server starts with an HTTP port configured
- **THEN** it SHALL listen on that port and serve all endpoints

#### Scenario: SSE event stream
- **WHEN** a client connects to `GET /events`
- **THEN** it SHALL receive a Server-Sent Events stream of all `ServerEvent` instances
- **AND** a heartbeat event every 10 seconds

#### Scenario: Streaming message endpoint
- **WHEN** a client POSTs to `/sessions/:id/message/stream` with content
- **THEN** the server SHALL start a turn and return 200 with `Content-Type: application/x-ndjson`
- **AND** each `Event` SHALL be serialized as one JSON line followed by `\n`
- **AND** events SHALL also flow to all SSE subscribers

#### Scenario: Send message via HTTP
- **WHEN** a client POSTs to `/sessions/:id/message` with content
- **THEN** the server SHALL start a turn and return 202 Accepted
- **AND** events SHALL flow to all SSE subscribers

#### Scenario: Multiple HTTP clients
- **WHEN** two clients connect to the SSE endpoint
- **THEN** both SHALL receive all events independently

### Requirement: ServerStatus
The system SHALL define a `ServerStatus` struct:
- `providers: Vec<ProviderInfo>` — provider metadata with available models
- `tools: Vec<String>` — tool names
- `active_sessions: usize` — total session count across all projects
- `active_turns: Vec<Ulid>` — session IDs with active turns

#### Scenario: Status query
- **WHEN** `GET /status` is called
- **THEN** it SHALL return current engine state including provider info

### Requirement: Implementation Status
The `brain-cli` dual-mode experience (`brain` + optional `brain serve`) SHALL be tracked in `add-brain-cli` and `add-tui-transport` change specs.
The current `brain-server` implementation SHALL define and expose only server API behavior.

#### Scenario: Server-only scope
- **WHEN** the current runtime is started from this change set
- **THEN** the server SHALL expose only server API behavior
