# brain-stores Specification

## Purpose
Project, session, and message persistence. Defines the `ProjectStore`, `SessionStore`, and `MessageStore` sub-traits and the composite `Store` supertrait. Provides `InMemoryStore` (ephemeral) and `FileStore` (filesystem-backed, human-readable JSON/JSONL) implementations.
## Requirements
### Requirement: Store Trait Decomposition
The system SHALL define three sub-traits for persistence, each independently usable as a trait bound:

- **`ProjectStore`**: CRUD for projects — `project_create`, `project_get`, `project_list`, `project_update`, `project_delete`
- **`SessionStore`**: CRUD for sessions scoped to a project — `session_create(project_id)`, `session_get`, `session_list(project_id)`, `session_update`, `session_delete`
- **`MessageStore`**: append and list messages for a session — `message_append`, `message_list`

A composite **`Store`** supertrait SHALL inherit all three sub-traits. A blanket implementation SHALL auto-derive `Store` for any type implementing all three sub-traits.

```rust
pub trait Store: ProjectStore + SessionStore + MessageStore {}
impl<T: ProjectStore + SessionStore + MessageStore> Store for T {}
```

All methods use the `model_verb` naming convention (e.g., `session_create` not `create_session`).

#### Scenario: Store supertrait blanket impl
- **WHEN** a type implements `ProjectStore`, `SessionStore`, and `MessageStore`
- **THEN** it SHALL automatically implement the `Store` supertrait

### Requirement: Project Entity
A `Project` struct SHALL represent a workspace/project context with:
- `id: ProjectId` (type alias for `Ulid`)
- `name: Option<String>`
- `root: Option<PathBuf>` (filesystem path, `None` for web-only or test projects)
- `config: ProjectConfig` (resolved configuration for the project)
- `created_at: DateTime<Utc>`
- `updated_at: DateTime<Utc>`

A `ProjectUpdate` struct SHALL allow updating `name` and `config` fields.

A `ProjectConfig` struct SHALL wrap `AgentConfig` (with `#[serde(flatten)]`) and serve as the extensible container for project-level configuration.

#### Scenario: Project creation with defaults
- **WHEN** `Project::with_defaults(name)` is called
- **THEN** it SHALL return a project with a unique ID, default config, and current timestamps

### Requirement: Session Scoping
A `Session` struct SHALL include a `project_id: ProjectId` field. Sessions are always scoped to a project. `session_create` takes a `project_id` parameter and `session_list` filters by `project_id`.

#### Scenario: Create session in project
- **WHEN** `session_create(project_id)` is called
- **THEN** it SHALL return a new `Session` with that `project_id`, a unique ID, no title, and current timestamps

#### Scenario: List sessions filters by project
- **WHEN** `session_list(project_id)` is called
- **THEN** it SHALL return only sessions belonging to that project, sorted by `updated_at` descending

### Requirement: ProjectStore Operations
The `ProjectStore` trait SHALL provide full CRUD operations for projects: create, get, list, update, and delete.

#### Scenario: Create and get project
- **WHEN** `project_create(project)` is called
- **THEN** it SHALL persist and return the project

#### Scenario: List projects
- **WHEN** multiple projects exist
- **THEN** `project_list()` SHALL return all projects sorted by `updated_at` descending

#### Scenario: Update project
- **WHEN** `project_update(id, ProjectUpdate { name: Some("New Name"), .. })` is called
- **THEN** `project_get(id)` SHALL return the project with the updated name and a new `updated_at`

#### Scenario: Delete project cascades
- **WHEN** `project_delete(id)` is called
- **THEN** the project and all its sessions and messages SHALL be removed

### Requirement: SessionStore Operations
The `SessionStore` trait SHALL provide CRUD operations for sessions scoped to a project: create, get, list, update, and delete.

#### Scenario: Update session title
- **WHEN** `session_update(id, SessionUpdate { title: Some("My Chat") })` is called
- **THEN** `session_get(id)` SHALL return a session with that title and an updated `updated_at`

#### Scenario: Delete session
- **WHEN** a session is deleted
- **THEN** `session_get(id)` SHALL return an error
- **AND** `message_list(id)` SHALL return an empty Vec

### Requirement: MessageStore Operations
The `MessageStore` trait SHALL provide append and list operations for messages within a session.

#### Scenario: Append and list messages
- **WHEN** messages are appended to a session via `message_append`
- **THEN** `message_list(session_id)` SHALL return all messages in order

### Requirement: InMemoryStore
The system SHALL include an `InMemoryStore` implementation in the `brain-stores` crate. It holds projects, sessions, and messages in `HashMap`s behind async-compatible `RwLock`s. It requires no external database and is suitable for testing and CLI use.

#### Scenario: In-memory persistence
- **WHEN** projects/sessions/messages are stored in InMemoryStore
- **THEN** they SHALL be retrievable until the process exits

#### Scenario: No persistence across restarts
- **WHEN** the process exits and restarts
- **THEN** InMemoryStore SHALL contain no data

### Requirement: FileStore
The system SHALL include a `FileStore` implementation in the `brain-stores` crate. It persists projects, sessions, and messages to the filesystem using a project-scoped directory layout. It requires no external database and produces human-readable files.

The directory layout SHALL be:
```
{root}/projects/{project_id}/project.json
{root}/projects/{project_id}/sessions/{ulid}/session.json
{root}/projects/{project_id}/sessions/{ulid}/messages.jsonl
```

- `project.json` — pretty-printed JSON of the `Project` struct, rewritten on updates
- `session.json` — pretty-printed JSON of the `Session` struct, rewritten on updates
- `messages.jsonl` — one JSON-serialized `Message` per line, append-only

#### Scenario: Filesystem persistence
- **WHEN** projects/sessions/messages are stored in FileStore
- **THEN** they SHALL survive process restarts
- **AND** the files SHALL be human-readable JSON/JSONL

#### Scenario: Append-only messages
- **WHEN** `message_append` is called
- **THEN** new messages SHALL be appended to `messages.jsonl` without rewriting existing lines

#### Scenario: Project directory lifecycle
- **WHEN** `project_create` is called
- **THEN** a new project directory SHALL be created with `project.json` and a `sessions/` subdirectory
- **WHEN** `project_delete` is called
- **THEN** the entire project directory (including all sessions) SHALL be removed

#### Scenario: Session directory lifecycle
- **WHEN** `session_create` is called
- **THEN** a new session directory SHALL be created under the project with `session.json` and an empty `messages.jsonl`
- **WHEN** `session_delete` is called
- **THEN** the entire session directory SHALL be removed

#### Scenario: Caller controls root path
- **WHEN** `FileStore::new(root)` is called
- **THEN** the store SHALL create the `projects/` subdirectory if it does not exist
- **AND** all data SHALL be scoped under that root path

### Requirement: CredentialStore Multi-Credential Support
The `CredentialStore` trait SHALL support multiple credentials per provider, keyed by `(provider_name, credential_id)` composite key. Methods:
- `credential_save(provider_name, entry: &CredentialEntry)` — upsert a credential entry (uses `entry.id` as credential_id)
- `credential_load(provider_name, credential_id)` — load a single credential by composite key
- `credential_load_all(provider_name)` — load all credentials for a provider
- `credential_delete(provider_name, credential_id)` — delete a single credential
- `credential_update_health(provider_name, credential_id, health)` — update health metadata without replacing the credential
- `credential_list()` — list all (provider_name, CredentialEntry) pairs across all providers

The `Store` supertrait SHALL include `CredentialStore`.

#### Scenario: Save and load multiple credentials for one provider
- **WHEN** two CredentialEntries with different ids are saved for provider "openai"
- **THEN** `credential_load("openai", id1)` SHALL return the first entry
- **AND** `credential_load("openai", id2)` SHALL return the second entry
- **AND** `credential_load_all("openai")` SHALL return both entries

#### Scenario: Update health without replacing credential
- **WHEN** `credential_update_health("openai", "key-1", health)` is called
- **THEN** `credential_load("openai", "key-1")` SHALL return the entry with updated health
- **AND** the credential itself (api_key or OAuth tokens) SHALL be unchanged

#### Scenario: Delete single credential
- **WHEN** `credential_delete("openai", "key-1")` is called
- **THEN** `credential_load("openai", "key-1")` SHALL return `Ok(None)`
- **AND** other credentials for "openai" SHALL be unaffected

#### Scenario: List across providers
- **WHEN** credentials exist for "openai" and "anthropic"
- **THEN** `credential_list()` SHALL return entries for both providers

### Requirement: InMemoryStore Multi-Credential
The `InMemoryStore` SHALL store credentials in a `HashMap<(String, String), CredentialEntry>` keyed by `(provider_name, credential_id)`.

#### Scenario: Multi-credential persistence
- **WHEN** multiple credentials are saved for the same provider in InMemoryStore
- **THEN** they SHALL all be retrievable individually and via `load_all`

### Requirement: FileStore Multi-Credential
The `FileStore` SHALL persist credentials under `{root}/credentials/{provider_name}/{credential_id}.json`. Each credential file SHALL contain a single JSON-serialized `CredentialEntry`.

#### Scenario: Multi-credential file layout
- **WHEN** credentials "key-1" and "key-2" are saved for provider "openai"
- **THEN** files SHALL exist at `{root}/credentials/openai/key-1.json` and `{root}/credentials/openai/key-2.json`

#### Scenario: Delete removes file
- **WHEN** `credential_delete("openai", "key-1")` is called
- **THEN** the file `{root}/credentials/openai/key-1.json` SHALL be removed


