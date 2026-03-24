## ADDED Requirements

### Requirement: Provider-OpenAI Owns The Responses Client

The system SHALL provide a standalone `provider-openai` crate that owns the
OpenAI Responses request/response/event model and the concrete client used to
call the OpenAI Responses API.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Standalone crate exposes a concrete client
- **WHEN** a caller depends on `provider-openai`
- **THEN** it SHALL be able to construct a concrete client from that crate
- **AND** call the OpenAI Responses API without importing `brain-types`

#### Scenario: Standalone crate owns Responses types
- **WHEN** a caller needs request, response, or stream event types for the OpenAI Responses API
- **THEN** those types SHALL be provided by `provider-openai`
- **AND** they SHALL not be re-exported from `brain-types`

### Requirement: Provider-OpenAI Supports Full Responses Resource Operations

The `provider-openai` client SHALL support the full Responses resource family
currently implemented on the v2 path.

At minimum this SHALL include:

- create response
- stream response over SSE
- stream response over WebSocket
- retrieve response
- stream retrieve response
- cancel response
- delete response
- compact response
- list response input items
- count response input tokens

#### Scenario: Client creates and streams responses
- **WHEN** a caller uses the standalone client to create or stream a response
- **THEN** the client SHALL return typed response objects or typed stream events from `provider-openai`

#### Scenario: Client supports resource follow-up endpoints
- **WHEN** a caller retrieves, cancels, deletes, compacts, lists input items, or counts input tokens
- **THEN** the client SHALL expose typed methods for those operations
