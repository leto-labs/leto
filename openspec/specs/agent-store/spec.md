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

