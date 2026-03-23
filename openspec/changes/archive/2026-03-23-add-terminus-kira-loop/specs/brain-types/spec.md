## ADDED Requirements

### Requirement: Messages Support Structured Multimodal Content

The shared message model SHALL support both plain-text content and structured
content parts.

At minimum, the structured content form SHALL support:

- text parts
- image URL parts

Text-only callers SHALL remain source-compatible through helper constructors.

#### Scenario: Text message remains simple

- **WHEN** a caller creates a text-only user or assistant message
- **THEN** the message SHALL preserve that content without requiring structured
  parts

#### Scenario: Multimodal message carries text and image

- **WHEN** a caller constructs a message with both text and image URL parts
- **THEN** the message SHALL preserve the part ordering and part payloads for
  provider serialization
