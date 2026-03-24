## ADDED Requirements

### Requirement: Shared Provider SDK Crate

The system SHALL provide a standalone `provider` crate that owns the shared
provider trait, request model, streamed event model, capabilities, and mock
implementation.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Shared crate exposes a reusable provider contract

- **WHEN** a caller depends on `provider`
- **THEN** it SHALL be able to construct shared requests and consume shared
  provider events without importing `brain-types`

### Requirement: Shared Provider Trait Is Stream-First

The `provider` crate SHALL define a stream-first `Provider` trait for model
inference.

The trait SHALL accept a shared request object and return a stream of shared
provider events.

#### Scenario: Shared provider returns block-oriented events

- **WHEN** a provider streams a response
- **THEN** the stream SHALL emit response start, block lifecycle, usage, and
  completion events

### Requirement: Shared Request Model Supports Structured Transcripts

The shared request model SHALL support:

- model selection
- transcript messages
- structured content blocks
- tool definitions
- inference options

#### Scenario: Shared request carries text, images, and tool state

- **WHEN** a caller builds a request with text, image, tool-call, and tool-result
  blocks
- **THEN** the shared request model SHALL represent those blocks without forcing
  provider-specific wire types

### Requirement: Mock Provider Implements The Shared Trait

The `provider` crate SHALL include a `MockProvider` that implements the shared
trait without network or model dependencies.

#### Scenario: Mock provider streams a text block

- **WHEN** a shared mock provider receives a text request
- **THEN** it SHALL emit shared block lifecycle events and a completion event

#### Scenario: Mock provider can simulate tool calls

- **WHEN** the request text starts with `tool:` and tools are available
- **THEN** the mock provider SHALL emit a shared tool-call block
