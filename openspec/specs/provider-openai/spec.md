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
