## ADDED Requirements

### Requirement: OpenAI-Compatible Provider Supports Structured Message Content

The OpenAI-compatible provider request builders SHALL support both text-only
messages and structured multimodal messages.

At minimum, supported structured parts SHALL include:

- text
- image URL

#### Scenario: Chat Completions serializes text-plus-image message

- **WHEN** the provider is building a Chat Completions request from a user
  message containing text and image URL parts
- **THEN** it SHALL serialize the request using the API's content-part shape
  rather than flattening the message into a single string

#### Scenario: Responses serializes text-plus-image message

- **WHEN** the provider is building a Responses request from a user message
  containing text and image URL parts
- **THEN** it SHALL serialize the request using the API's structured input
  message format
