# agent-store Specification

## Purpose
TBD - created by archiving change add-agent-core-and-store. Update Purpose after archive.
## Requirements
### Requirement: Agent Store Must Persist Durable V2 Records

The system MUST provide a dedicated `agent-store` crate that persists durable
project, session, message, credential, and trajectory records for the v2 stack.

#### Scenario: Persist canonical transcript messages
- **WHEN** a caller stores transcript history for a session
- **THEN** the store persists `StoredMessage` records
- **AND** each `StoredMessage` wraps canonical `provider::Message` content rather than a duplicate transcript schema

#### Scenario: Persist ATIF trajectories without redefining the schema
- **WHEN** a caller stores a trajectory for a session
- **THEN** the store persists canonical `atif` trajectory types
- **AND** the store rejects invalid trajectories as invalid input

### Requirement: Agent Store Must Offer Memory And File Implementations

The system MUST provide at least in-memory and file-backed implementations of
the `agent-store` traits.

#### Scenario: File store survives process restart
- **WHEN** a caller writes projects, sessions, and messages to `FileStore`
- **AND** later reopens the same store root
- **THEN** the stored records remain available

#### Scenario: File store removes stale persisted files
- **WHEN** a caller deletes a session or replaces persisted data
- **THEN** reopening the file-backed store does not resurrect deleted stale JSON files

### Requirement: Agent Store Persists Provider Credential Metadata

The `agent-store` crate SHALL persist provider credentials with enough metadata
to support OAuth refresh, account-aware provider requests, and credential
health tracking.

This SHALL include:

- multiple credentials per provider under a composite key
- API-key and OAuth-backed credential bodies
- OAuth access token, refresh token, client id, token endpoint, expiry, token
  type, scopes, and optional account context
- credential health state that can be updated independently of the credential
  body

#### Scenario: Stored OAuth credential round-trips with refresh metadata

- **WHEN** a caller persists and later reloads an OAuth-backed credential
- **THEN** the stored record SHALL preserve refresh, expiry, and account-aware
  metadata required by outer application layers

#### Scenario: Credential health updates do not replace the credential body

- **WHEN** a caller updates only the health state for one stored credential
- **THEN** the store SHALL preserve the existing API key or OAuth tokens
- **AND** only mutate the health metadata

### Requirement: Agent Store Persists Project Bootstrap Defaults

The `agent-store` crate SHALL persist the project-level defaults required for
bootstrap parity on the refactored stack.

This SHALL include:

- project-level system prompt guidance
- default loop selection
- default provider and model selection
- runtime-native request and hardening defaults used by `agent-core`

#### Scenario: Project defaults round-trip through durable storage

- **WHEN** a caller persists and later reloads a project with prompt, loop,
  provider, model, and runtime defaults
- **THEN** the reloaded project SHALL preserve those defaults without requiring
  a separate config-resolution pass

#### Scenario: Missing new project-default fields stay optional

- **WHEN** a caller reloads a project record created before the new parity
  fields existed
- **THEN** the missing fields SHALL deserialize as empty or default values
- **AND** the record SHALL remain readable without migration-time failure

