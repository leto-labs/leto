## MODIFIED Requirements

### Requirement: Provider-OpenAI Owns The Responses Client

The system SHALL provide a standalone `provider-openai` crate that owns the
OpenAI Responses request/response/event model and the concrete client used to
call the OpenAI Responses API.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.
It MAY depend on the standalone shared `provider` crate to implement the shared
provider SDK trait and publish shared model catalog metadata.

#### Scenario: Standalone crate exposes a concrete client

- **WHEN** a caller depends on `provider-openai`
- **THEN** it SHALL be able to construct a concrete client from that crate
- **AND** call the OpenAI Responses API without importing `brain-types`

#### Scenario: Standalone crate owns Responses types

- **WHEN** a caller needs request, response, or stream event types for the OpenAI
  Responses API
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

### Requirement: Provider-OpenAI Exposes Explicit Modern API Surfaces

The `provider-openai` crate SHALL expose explicit modern API surfaces for:

- `responses`
- `chat_completions`

#### Scenario: Top-level client exposes both surfaces

- **WHEN** a caller creates a `provider_openai::Client`
- **THEN** the client SHALL expose a Responses surface
- **AND** the client SHALL expose a Chat Completions surface

### Requirement: Responses Surface Preserves Existing Resource Support

The Responses surface SHALL support the standalone client operations already
implemented in the crate.

#### Scenario: Responses supports create and resource operations

- **WHEN** a caller uses the Responses surface
- **THEN** it SHALL support create, retrieve, delete, cancel, compact,
  input-item listing, and input-token counting
- **AND** it SHALL support SSE streaming
- **AND** it SHALL support WebSocket streaming for create

### Requirement: Chat Completions Surface Supports Message-Oriented Create And Stream

The Chat Completions surface SHALL support the message-oriented modern chat
endpoint, not legacy prompt-based completions.

#### Scenario: Chat Completions create

- **WHEN** a caller sends a non-streaming chat-completions request
- **THEN** the client SHALL call `/chat/completions`
- **AND** it SHALL return a parsed chat completion object

#### Scenario: Chat Completions SSE stream

- **WHEN** a caller sends a streaming chat-completions request
- **THEN** the client SHALL call `/chat/completions` with streaming enabled
- **AND** it SHALL return a stream of parsed chat-completion chunks

### Requirement: Chat Completions Supports Structured Message Content

The Chat Completions request builder SHALL support text-only messages and
structured multimodal message parts.

#### Scenario: Text and image URL content parts serialize correctly

- **WHEN** a caller constructs a user message with text and image URL parts
- **THEN** the client SHALL serialize that message using the chat-completions
  content-part format rather than flattening it into a single string

### Requirement: Provider-OpenAI Exposes A Shared Provider Implementation

The `provider-openai` crate SHALL expose a provider implementation for the
shared `provider::Provider` trait while preserving its protocol-native client.

#### Scenario: OpenAI acts as a provider plugin

- **WHEN** a caller constructs the shared OpenAI provider implementation
- **THEN** it SHALL accept shared provider requests
- **AND** stream shared provider events translated from OpenAI Responses events

## ADDED Requirements

### Requirement: Provider-OpenAI Publishes Generated Typed Presets

The `provider-openai` crate SHALL publish a typed preset/catalog module for the
supported OpenAI-compatible providers.

Each preset SHALL preserve:

- provider/display name
- base URL
- env key
- default model id
- supported API surfaces
- rich shared model catalog metadata

#### Scenario: Preset builds a config without `brain-*` types

- **WHEN** a caller selects a built-in OpenAI-compatible preset
- **THEN** it SHALL be able to build a `provider_openai::Config` using only
  `provider-openai` and `provider` types

### Requirement: Provider-OpenAI Presets Are Generated From models.dev

The system SHALL provide a workspace Rust generator that emits the checked-in
`provider-openai` preset modules from the managed local `models.dev` checkout.

#### Scenario: Generate provider-openai presets from models.dev

- **WHEN** the generator runs
- **THEN** it SHALL read the configured provider allowlist from the local
  `models.dev` checkout
- **AND** emit the checked-in preset modules under `provider-openai`
- **AND** generate model catalogs using the shared `provider` model metadata

#### Scenario: Verify generated provider-openai presets are current

- **WHEN** the generator runs with `--check`
- **THEN** it SHALL fail if any checked-in generated preset file differs from
  the current generated output

### Requirement: Provider-OpenAI Smoke Tests Cover The Built-In Preset Matrix

The `provider-openai` crate SHALL include env-gated smoke coverage across the
built-in OpenAI-compatible preset matrix.

#### Scenario: Smoke tests skip presets without credentials

- **WHEN** a preset smoke test runs and the preset's declared env var is absent
- **THEN** the test SHALL skip cleanly for that preset

#### Scenario: Smoke tests use the best supported API surface

- **WHEN** a preset smoke test runs for a preset with credentials
- **THEN** it SHALL issue a basic inference request against a supported API
  surface for that preset
