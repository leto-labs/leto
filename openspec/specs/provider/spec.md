# provider Specification

## Purpose
TBD - created by archiving change add-provider-sdk. Update Purpose after archive.
## Requirements
### Requirement: Shared Provider SDK Crate

The system SHALL provide a standalone `provider` crate that owns the shared
provider trait, request model, streamed event model, capabilities, model
catalog metadata, and mock implementation.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Shared crate exposes reusable provider and model metadata

- **WHEN** a caller depends on `provider`
- **THEN** it SHALL be able to construct shared requests, consume shared
  provider events, and inspect rich shared model metadata without importing
  `brain-types`

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

### Requirement: Shared Provider Crate Owns Rich Model Catalog Metadata

The shared `provider` crate SHALL define reusable model catalog types rich
enough to preserve model identity, modalities, reasoning levels, pricing,
limits, and lifecycle metadata.

#### Scenario: Shared model info preserves rich catalog data

- **WHEN** a plugin publishes a model catalog through the shared provider SDK
- **THEN** each shared model entry SHALL be able to preserve model id, display
  name, modalities, reasoning-level data, pricing, limits, and lifecycle
  metadata

### Requirement: Provider Info Identifies The Default Model By Catalog Reference

Shared provider metadata SHALL identify the default model by id/reference while
also exposing the provider's full shared model catalog.

#### Scenario: Default model resolves from the shared catalog

- **WHEN** a provider publishes a default model id and a matching shared model
  catalog entry
- **THEN** callers SHALL be able to resolve the default model metadata from the
  shared catalog without duplicating the full model object in `ProviderInfo`

### Requirement: Shared Tool Definitions May Preserve Output Schemas

The shared `provider` crate MUST allow tool definitions to carry an optional
output schema in addition to the input schema used for provider tool
registration.

#### Scenario: Shared tool definition round-trips with output schema
- **WHEN** a caller constructs a shared tool definition with an output schema
- **THEN** the shared type preserves that schema during serialization and deserialization
- **AND** provider adapters may ignore the output schema on the wire without rejecting the definition

