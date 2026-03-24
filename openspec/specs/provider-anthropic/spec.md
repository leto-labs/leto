# provider-anthropic Specification

## Purpose
Standalone Anthropic wire client for the Messages API, including typed request,
response, and stream event models, without depending on `brain-*` crates.
## Requirements
### Requirement: Provider-Anthropic Owns The Messages Client

The system SHALL provide a standalone `provider-anthropic` crate that owns the
Anthropic Messages request/response/event model and the concrete client used to
call the Anthropic Messages API.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Standalone crate exposes a concrete client
- **WHEN** a caller depends on `provider-anthropic`
- **THEN** it SHALL be able to construct a concrete client from that crate
- **AND** call the Anthropic Messages API without importing `brain-types`

#### Scenario: Standalone crate owns Messages types
- **WHEN** a caller needs request, response, or stream event types for the
  Anthropic Messages API
- **THEN** those types SHALL be provided by `provider-anthropic`
- **AND** they SHALL not be re-exported from `brain-types`

### Requirement: Provider-Anthropic Exposes An Explicit Messages Surface

The `provider-anthropic` crate SHALL expose an explicit Messages surface
through a top-level client façade.

#### Scenario: Top-level client exposes Messages
- **WHEN** a caller creates a `provider_anthropic::Client`
- **THEN** the client SHALL expose a Messages surface

### Requirement: Messages Supports Create And SSE Stream

The Messages surface SHALL support non-streaming create and SSE streaming
create for Anthropic's `/v1/messages` endpoint.

#### Scenario: Messages create
- **WHEN** a caller sends a non-streaming message request
- **THEN** the client SHALL call `/v1/messages`
- **AND** it SHALL return a parsed message object

#### Scenario: Messages SSE stream
- **WHEN** a caller sends a streaming message request
- **THEN** the client SHALL call `/v1/messages` with streaming enabled
- **AND** it SHALL return a stream of parsed message events

### Requirement: Messages Supports Structured Content And Tool Use

The Messages request builder SHALL support text-only messages and structured
content needed for multimodal turns and tool use.

At minimum the supported structured content SHALL include:

- text blocks
- image blocks
- tool-use blocks
- tool-result blocks

#### Scenario: Text-plus-image content serializes correctly
- **WHEN** a caller constructs a user message with text and image content
- **THEN** the client SHALL serialize the message using Anthropic's structured
  content-block format rather than flattening it to a string

#### Scenario: Tool result content serializes correctly
- **WHEN** a caller constructs a user message containing a tool result
- **THEN** the client SHALL serialize that block using Anthropic's
  `tool_result` content-block format

### Requirement: Messages Stream Preserves Event Families

The Messages SSE parser SHALL preserve Anthropic's message/content event
families instead of collapsing them into a text-only stream.

#### Scenario: Text delta stream events are typed
- **WHEN** the stream contains `content_block_delta` text events
- **THEN** the client SHALL return typed delta events

#### Scenario: Message lifecycle events are typed
- **WHEN** the stream contains message start, delta, or stop events
- **THEN** the client SHALL return typed message lifecycle events
