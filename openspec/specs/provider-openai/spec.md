# provider-openai Specification

## Purpose
Standalone OpenAI-compatible wire client for modern OpenAI API surfaces,
including Responses and Chat Completions, without depending on `brain-*`
crates.
## Requirements
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
- **AND** stream shared provider events translated from the API surface selected
  by the resolved OpenAI-compatible configuration

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
- **AND** format the emitted Rust source before writing it

#### Scenario: Verify generated provider-openai presets are current

- **WHEN** the generator runs with `--check`
- **THEN** it SHALL fail if any checked-in generated preset file differs from
  the current formatted generated output

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

### Requirement: Shared OpenAI Provider Supports Every Declared API Surface

The shared `OpenAiProvider` implementation SHALL support every API surface
declared by its resolved `provider-openai::Config`.

#### Scenario: Responses-backed provider stream

- **WHEN** the resolved surface is `Responses`
- **THEN** the shared adapter SHALL stream shared provider events translated
  from the Responses API

#### Scenario: Chat-completions-backed provider stream

- **WHEN** the resolved surface is `ChatCompletions`
- **THEN** the shared adapter SHALL stream shared provider events translated
  from Chat Completions streaming chunks

#### Scenario: Unsupported forced surface errors cleanly

- **WHEN** a config forces an API surface that the preset/config does not
  support
- **THEN** the shared adapter SHALL return a configuration/runtime error rather
  than silently falling back

### Requirement: Shared OpenAI Provider Capabilities Reflect The Active Surface

The shared OpenAI provider metadata SHALL advertise capabilities for the active
resolved surface rather than the union across all possible surfaces.

#### Scenario: Forced chat-completions config narrows capabilities

- **WHEN** a config resolves to `ChatCompletions`
- **THEN** `OpenAiProvider::info()` SHALL report the capability set supported by
  Chat Completions

#### Scenario: Forced responses config advertises responses capabilities

- **WHEN** a config resolves to `Responses`
- **THEN** `OpenAiProvider::info()` SHALL report the capability set supported by
  Responses

### Requirement: Smoke Tests Cover Every Supported Preset-Surface Combination

Wire-client and shared-provider smoke coverage SHALL exercise every declared
supported API surface for each built-in preset with available credentials.

#### Scenario: Wire smoke iterates all declared surfaces

- **WHEN** a preset advertises more than one supported API surface
- **THEN** the wire smoke suite SHALL test each declared surface explicitly

#### Scenario: Shared adapter smoke iterates all declared surfaces

- **WHEN** a preset advertises more than one supported API surface
- **THEN** the shared adapter smoke suite SHALL test each declared surface
  explicitly

#### Scenario: Declared incompatible surface fails the smoke suite

- **WHEN** a configured preset/surface combination fails due to parser or
  compatibility drift
- **THEN** the smoke suite SHALL fail for that combination instead of treating
  it as an optional skip

### Requirement: Provider-OpenAI Supports A Standalone OAuth Surface

The `provider-openai` crate SHALL expose a standalone OAuth-backed provider
surface in addition to its direct API-key configuration path.

This SHALL include:

- browser and device login helpers
- refresh-token-based access-token renewal
- an OAuth-backed shared-provider adapter
- account-aware request shaping when required by the OpenAI OAuth backend

#### Scenario: Caller obtains persistence-ready OAuth credentials

- **WHEN** a caller completes a browser or device OAuth flow
- **THEN** `provider-openai` SHALL return credentials suitable for persistence
  by outer application layers

#### Scenario: OAuth-backed provider attaches account-aware headers

- **WHEN** the selected OAuth credential carries account context
- **THEN** the OAuth-backed provider SHALL attach the required account-aware
  request metadata in addition to bearer authentication

### Requirement: Provider-OpenAI Supports Shared Credential Pools

The `provider-openai` crate SHALL support managed credentials through the
shared `provider::CredentialPool`.

The shared OpenAI adapter and the OAuth-backed adapter SHALL both be
constructible from the shared pool while remaining usable without it through
their direct standalone constructors.

#### Scenario: Shared OpenAI adapter resolves credentials from a pool

- **WHEN** a caller constructs `OpenAiProvider` from the shared credential pool
- **THEN** each request SHALL resolve auth material from that pool
- **AND** terminal success or failure SHALL be reported back to the pool

#### Scenario: OAuth-backed adapter resolves credentials from a pool

- **WHEN** a caller constructs `OpenAiOAuthProvider` from the shared credential
  pool
- **THEN** each request SHALL resolve bearer authentication and account
  metadata from that pool
- **AND** terminal success or failure SHALL be reported back to the pool

