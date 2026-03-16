# brain-types Specification

## Purpose
Core abstraction for LLM inference. Defines the `Provider` trait, `ChatStream` type, and `MockProvider` for testing.
## Requirements
### Requirement: Provider Trait
The system SHALL define a `Provider` trait that abstracts LLM inference behind a single async method. Implementations MUST accept a slice of messages and tool definitions, and return a stream of response chunks.

#### Scenario: Mock provider echoes input
- **WHEN** a Provider receives messages with the last user message being "hello world"
- **THEN** it SHALL return a stream of token chunks containing "hello world"
- **AND** the stream SHALL terminate with a final chunk

#### Scenario: Provider returns tool calls
- **WHEN** a Provider receives messages and the model decides to call a tool
- **THEN** the stream SHALL include a chunk with tool call information (name + arguments)

### Requirement: ChatStream Type
The system SHALL define a `ChatStream` type as a pinned boxed stream of `Result<ChatChunk, BrainError>`. Each `ChatChunk` SHALL carry either a text delta, a tool call, usage stats, or a done signal.

#### Scenario: Stream yields text tokens
- **WHEN** the provider streams a text response
- **THEN** the stream SHALL yield one or more `ChatChunk::Delta` items containing text fragments

#### Scenario: Stream signals completion
- **WHEN** the provider finishes its response
- **THEN** the stream SHALL yield a `ChatChunk::Done` item

### Requirement: MockProvider
The system SHALL include a `MockProvider` that requires no API key, no network, and no model files. It SHALL echo the last user message as space-delimited token chunks with an optional configurable delay.

#### Scenario: Deterministic echo
- **WHEN** MockProvider receives messages ending with "hello world"
- **THEN** it SHALL stream tokens: "hello", " ", "world"

#### Scenario: Simulated tool call
- **WHEN** MockProvider receives a message starting with "tool:"
- **THEN** it SHALL return a tool call chunk for the "echo" tool


