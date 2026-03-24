## ADDED Requirements

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
