## MODIFIED Requirements

### Requirement: Shared Request Model Supports Structured Transcripts

The shared request model SHALL support:

- model selection
- transcript messages
- structured content blocks
- tool definitions
- inference options

Tool-call transcript blocks SHALL preserve a stable transcript-facing id and
MAY also preserve a distinct execution-facing call id when a provider surface
requires both identities.

#### Scenario: Shared request carries text, images, and tool state

- **WHEN** a caller builds a request with text, image, tool-call, and
  tool-result blocks
- **THEN** the shared request model SHALL represent those blocks without
  forcing provider-specific wire types

#### Scenario: Shared tool-call transcript preserves transcript and execution ids

- **WHEN** a caller replays a transcript that needs one id for provider memory
  and a different id for tool execution follow-up
- **THEN** the shared request model SHALL preserve both ids on the tool-call
  block
- **AND** callers that only need one id SHALL remain able to omit the optional
  execution id
