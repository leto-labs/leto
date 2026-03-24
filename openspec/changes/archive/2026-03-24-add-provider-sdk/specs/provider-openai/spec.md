## MODIFIED Requirements

### Requirement: Provider-OpenAI Owns The Responses Client

The system SHALL provide a standalone `provider-openai` crate that owns the
OpenAI Responses request/response/event model and the concrete client used to
call the OpenAI Responses API.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.
It MAY depend on the standalone shared `provider` crate to implement the shared
provider SDK trait.

#### Scenario: Standalone crate exposes a concrete client

- **WHEN** a caller depends on `provider-openai`
- **THEN** it SHALL be able to construct a concrete client from that crate
- **AND** call the OpenAI Responses API without importing `brain-types`

#### Scenario: Standalone crate owns Responses types

- **WHEN** a caller needs request, response, or stream event types for the OpenAI
  Responses API
- **THEN** those types SHALL be provided by `provider-openai`
- **AND** they SHALL not be re-exported from `brain-types`

## ADDED Requirements

### Requirement: Provider-OpenAI Exposes A Shared Provider Implementation

The `provider-openai` crate SHALL expose a provider implementation for the
shared `provider::Provider` trait while preserving its protocol-native client.

#### Scenario: OpenAI acts as a provider plugin

- **WHEN** a caller constructs the shared OpenAI provider implementation
- **THEN** it SHALL accept shared provider requests
- **AND** stream shared provider events translated from OpenAI Responses events
