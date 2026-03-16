# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: OpenAiConfig
The system SHALL define an `OpenAiConfig` struct with `api_key`, `base_url`, and `default_model` fields. The library SHALL NOT read environment variables — all config is passed as data by the caller.

#### Scenario: Default config
- **WHEN** `OpenAiConfig::new("sk-test")` is called
- **THEN** `base_url` SHALL be `"https://api.openai.com/v1"` and `default_model` SHALL be `"gpt-4o-mini"`

#### Scenario: Custom endpoint
- **WHEN** `OpenAiConfig::new("key").with_base_url("http://localhost:11434/v1")` is called
- **THEN** requests SHALL be sent to the custom URL

### Requirement: OpenAiProvider
The system SHALL implement the `Provider` trait for `OpenAiProvider`. It SHALL send streaming chat completion requests to an OpenAI-compatible API and return a `ChatStream`.

#### Scenario: Text response
- **WHEN** the API streams text deltas
- **THEN** the provider SHALL yield `ChatChunk::Delta` for each content fragment and `ChatChunk::Done` with usage at the end

#### Scenario: Tool call response
- **WHEN** the API streams tool call deltas across multiple SSE chunks
- **THEN** the provider SHALL accumulate arguments by tool call index and yield a single `ChatChunk::ToolCall` per tool call with the complete parsed arguments

#### Scenario: API error
- **WHEN** the API returns a non-200 status code
- **THEN** the provider SHALL return `BrainError::Inference` with the status code and response body

#### Scenario: Model selection
- **WHEN** `InferenceConfig.model` is `None`
- **THEN** the provider SHALL use `OpenAiConfig.default_model`
- **WHEN** `InferenceConfig.model` is `Some("gpt-4o")`
- **THEN** the provider SHALL use `"gpt-4o"`

### Requirement: Feature Gate
The `OpenAiProvider` SHALL be gated behind the `openai` feature flag (enabled by default). Building with `--no-default-features` SHALL not compile any HTTP dependencies.

#### Scenario: Feature disabled
- **WHEN** `brain-providers` is compiled with `--no-default-features`
- **THEN** `OpenAiProvider` and `OpenAiConfig` SHALL not be available and no HTTP dependencies SHALL be linked

