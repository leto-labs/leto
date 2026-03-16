# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## MODIFIED Requirements

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
