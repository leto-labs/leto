## MODIFIED Requirements

### Requirement: Agent Core Must Assemble The V2 Runtime Stack

The system MUST provide an `agent-core` crate that acts as the app-facing
composition boundary above `agent-runtime`.

#### Scenario: Default local assembly
- **WHEN** a caller builds the default local core surface
- **THEN** the core registers a default loop
- **AND** installs a native `agent-tools` tool executor
- **AND** requires at least one real registered or discovered provider

#### Scenario: Default local assembly rejects missing providers
- **WHEN** a caller builds the default local core surface with no registered providers and no discoverable credentials
- **THEN** the build fails with a no-providers error

#### Scenario: Credential-based provider discovery
- **WHEN** the backing store contains OpenAI-compatible API-key or OAuth-backed credentials
- **THEN** the core registers corresponding providers during default local assembly

## ADDED Requirements

### Requirement: Agent Core Activates Stored Credentials Into Shared Provider Pools

The `agent-core` crate SHALL activate persisted credentials into the shared
provider credential pool rather than owning a separate credential-selection
implementation.

This SHALL include:

- loading stored credentials into the shared pool before provider use
- refreshing expiring OpenAI OAuth credentials before activation
- persisting updated credential health from the shared pool back into the store
  after use

#### Scenario: Core refreshes expiring OAuth credentials before activation

- **WHEN** a stored OpenAI OAuth credential is near expiry before a turn starts
- **THEN** `agent-core` SHALL refresh that credential before activating it into
  the shared provider pool

#### Scenario: Core persists pool health back to the store

- **WHEN** a pool-backed provider marks a credential successful or failed during
  a turn
- **THEN** `agent-core` SHALL persist the resulting credential health state
  back into the store after the turn completes
