# brain-types Delta Spec

## ADDED Requirements

### Requirement: Dedicated Core Trait Modules
The system SHALL expose the remaining core runtime traits from dedicated
`brain-types` modules instead of a shared catch-all trait file.

The crate SHALL provide:
- `tool.rs` for `Tool` and tool-related data types
- `store.rs` for `ProjectStore`, `SessionStore`, `MessageStore`,
  `CredentialStore`, and `Store`
- `agent_loop.rs` for `EventStream` and `AgentLoop`

The crate root SHALL continue re-exporting these types so existing imports such
as `brain_types::Store` and `brain_types::AgentLoop` remain valid.

#### Scenario: Import store traits from dedicated module
- **WHEN** a caller imports `brain_types::store::{ProjectStore, Store}`
- **THEN** the store traits SHALL resolve from the dedicated `store` module

#### Scenario: Import agent loop traits from dedicated module
- **WHEN** a caller imports `brain_types::agent_loop::{AgentLoop, EventStream}`
- **THEN** the agent loop abstractions SHALL resolve from the dedicated
  `agent_loop` module

#### Scenario: Existing root-level imports continue to resolve
- **WHEN** a caller imports `brain_types::{Tool, Store, AgentLoop}`
- **THEN** those root-level re-exports SHALL continue to resolve
