# brain-providers-openai-api Specification

## Purpose
OpenAI-compatible API providers, provider presets, and request/streaming behavior for provider backends that call remote HTTP services.
## Requirements
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

### Requirement: OpenAiConfigPreset
The system SHALL define an `OpenAiConfigPreset` struct with `name`, `base_url`, `default_model`, and `env_key` fields (all `&'static str`). It SHALL provide an `into_config(api_key)` method that produces an `OpenAiConfig`, and a `from_env()` method that reads the API key from the environment variable named by `env_key` and returns `Some(OpenAiConfig)` if set or `None` if not.

#### Scenario: Preset to config
- **WHEN** `OpenAiConfigPreset::GROQ.into_config("sk-xxx")` is called
- **THEN** the resulting `OpenAiConfig` SHALL have `base_url` of `"https://api.groq.com/openai/v1"` and `default_model` of `"llama-3.3-70b-versatile"`

#### Scenario: from_env with key set
- **WHEN** the environment variable named by `preset.env_key` is set
- **THEN** `preset.from_env()` SHALL return `Some(OpenAiConfig)` with that key

#### Scenario: from_env without key
- **WHEN** the environment variable named by `preset.env_key` is not set
- **THEN** `preset.from_env()` SHALL return `None`

### Requirement: Built-in presets
The system SHALL provide `const` presets for at least: OpenAI, Groq, DeepSeek, Together, xAI, Fireworks, Mistral, OpenRouter, Ollama, Gemini, and MiniMax. An `ALL` slice SHALL list every built-in preset. A `by_name(name)` method SHALL return the matching preset by case-insensitive name lookup, or `None` if not found. Each preset SHALL include an `env_key` matching the conventional environment variable for that provider's API key.

#### Scenario: Lookup by name
- **WHEN** `OpenAiConfigPreset::by_name("groq")` is called
- **THEN** it SHALL return `Some` with the Groq preset

#### Scenario: Unknown name
- **WHEN** `OpenAiConfigPreset::by_name("unknown")` is called
- **THEN** it SHALL return `None`

#### Scenario: ALL contains all presets
- **WHEN** `OpenAiConfigPreset::ALL` is accessed
- **THEN** it SHALL contain at least 11 entries covering the built-in providers

### Requirement: Generated preset modules
The system SHALL provide a workspace Rust generator that emits the checked-in `OpenAiConfigPreset` Rust modules for supported OpenAI-compatible API providers from a managed local models.dev checkout. The generated output SHALL remain compatible with the public `OpenAiConfigPreset` API used by callers.

#### Scenario: Generate preset modules from models.dev
- **WHEN** the preset generator is run
- **THEN** it SHALL read the configured provider allowlist from the local models.dev checkout
- **AND** it SHALL write per-provider preset Rust modules under `crates/brain-providers/src/openai/presets/`
- **AND** those generated modules SHALL expose the same built-in preset constants through `OpenAiConfigPreset`

#### Scenario: Verify generated preset modules are current
- **WHEN** the preset generator is run in check mode
- **THEN** it SHALL fail if any checked-in generated preset module differs from the current generated output

### Requirement: Chat preset catalogs
The system SHALL generate model catalogs for OpenAI-compatible chat presets from the supported models.dev providers while filtering models that are not suitable for the chat-completions surface exposed by `OpenAiProvider`.

#### Scenario: Non-chat models are excluded
- **WHEN** a source provider includes embedding or other non-chat models
- **THEN** the generated preset catalog SHALL exclude those models from `OpenAiConfigPreset.models`

#### Scenario: Recent supported chat models are included
- **WHEN** the source provider includes newer supported chat models
- **THEN** the generated preset catalog SHALL include those model IDs in the corresponding preset

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

