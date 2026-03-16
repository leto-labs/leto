# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: Store Trait
The system SHALL define a `Store` trait with async methods for:
- **Sessions**: create, get, update, list, and delete sessions
- **Messages**: append messages to a session and load all messages for a session

A `Session` struct SHALL have `id: Ulid`, `title: Option<String>`, `created_at: DateTime<Utc>`, and `updated_at: DateTime<Utc>`.

A `SessionUpdate` struct SHALL allow updating the `title` field.

#### Scenario: Create and get session
- **WHEN** `create_session()` is called
- **THEN** it SHALL return a new `Session` with a unique ID, no title, and current timestamps

#### Scenario: Update session title
- **WHEN** `update_session(id, SessionUpdate { title: Some("My Chat") })` is called
- **THEN** `get_session(id)` SHALL return a session with that title and an updated `updated_at`

#### Scenario: List sessions
- **WHEN** multiple sessions have been created
- **THEN** `list_sessions()` SHALL return all sessions sorted by `updated_at` descending

#### Scenario: Delete session
- **WHEN** a session is deleted
- **THEN** `get_session(id)` SHALL return an error
- **AND** `load_messages(id)` SHALL return an empty Vec

#### Scenario: Append and load messages
- **WHEN** messages are appended to a session
- **THEN** `load_messages(session_id)` SHALL return all messages in order

### Requirement: InMemoryStore
The system SHALL include an `InMemoryStore` implementation in the `brain-stores` crate. It holds sessions and messages in a `HashMap` behind an async-compatible `RwLock`. It requires no external database and is suitable for testing and CLI use.

#### Scenario: In-memory persistence
- **WHEN** sessions/messages are stored in InMemoryStore
- **THEN** they SHALL be retrievable until the process exits

#### Scenario: No persistence across restarts
- **WHEN** the process exits and restarts
- **THEN** InMemoryStore SHALL contain no data

### Requirement: FileStore
The system SHALL include a `FileStore` implementation in the `brain-stores` crate. It persists sessions and messages to the filesystem using a directory-per-session layout. It requires no external database and produces human-readable files.

The directory layout SHALL be:
```
{root}/sessions/{ulid}/session.json
{root}/sessions/{ulid}/messages.jsonl
```

- `session.json` -- pretty-printed JSON of the `Session` struct, rewritten on updates
- `messages.jsonl` -- one JSON-serialized `Message` per line, append-only

#### Scenario: Filesystem persistence
- **WHEN** sessions/messages are stored in FileStore
- **THEN** they SHALL survive process restarts
- **AND** the files SHALL be human-readable JSON/JSONL

#### Scenario: Append-only messages
- **WHEN** `append_messages` is called
- **THEN** new messages SHALL be appended to `messages.jsonl` without rewriting existing lines

#### Scenario: Session directory lifecycle
- **WHEN** `create_session` is called
- **THEN** a new directory SHALL be created with `session.json` and an empty `messages.jsonl`
- **WHEN** `delete_session` is called
- **THEN** the entire session directory SHALL be removed

#### Scenario: Caller controls root path
- **WHEN** `FileStore::new(root)` is called
- **THEN** the store SHALL create the `sessions/` subdirectory if it does not exist
- **AND** all data SHALL be scoped under that root path
