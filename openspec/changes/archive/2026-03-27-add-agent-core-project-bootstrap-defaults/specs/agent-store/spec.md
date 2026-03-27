## ADDED Requirements

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
