## MODIFIED Requirements

### Requirement: Provider-Anthropic Owns The Messages Client

The system SHALL provide a standalone `provider-anthropic` crate that owns the
Anthropic Messages request/response/event model and the concrete client used to
call the Anthropic Messages API.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.
It MAY depend on the standalone shared `provider` crate to implement the shared
provider SDK trait.

#### Scenario: Standalone crate exposes a concrete client

- **WHEN** a caller depends on `provider-anthropic`
- **THEN** it SHALL be able to construct a concrete client from that crate
- **AND** call the Anthropic Messages API without importing `brain-types`

## ADDED Requirements

### Requirement: Provider-Anthropic Exposes A Shared Provider Implementation

The `provider-anthropic` crate SHALL expose a provider implementation for the
shared `provider::Provider` trait while preserving its protocol-native client.

#### Scenario: Anthropic acts as a provider plugin

- **WHEN** a caller constructs the shared Anthropic provider implementation
- **THEN** it SHALL accept shared provider requests
- **AND** stream shared provider events translated from Anthropic Messages SSE
  events
