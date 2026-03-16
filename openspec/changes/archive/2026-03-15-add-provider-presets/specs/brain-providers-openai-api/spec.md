# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: OpenAiConfigPreset
The system SHALL define an `OpenAiConfigPreset` struct with `name`, `base_url`, and `default_model` fields (all `&'static str`). It SHALL provide an `into_config(api_key)` method that produces an `OpenAiConfig`. This allows users to select a known provider without manually specifying base URLs and models.

#### Scenario: Preset to config
- **WHEN** `OpenAiConfigPreset::GROQ.into_config("sk-xxx")` is called
- **THEN** the resulting `OpenAiConfig` SHALL have `base_url` of `"https://api.groq.com/openai/v1"` and `default_model` of `"llama-3.3-70b-versatile"`

### Requirement: Built-in presets
The system SHALL provide `const` presets for at least: OpenAI, Groq, DeepSeek, Together, xAI, Fireworks, Mistral, OpenRouter, and Ollama. An `ALL` slice SHALL list every built-in preset. A `by_name(name)` method SHALL return the matching preset by case-insensitive name lookup, or `None` if not found.

#### Scenario: Lookup by name
- **WHEN** `OpenAiConfigPreset::by_name("groq")` is called
- **THEN** it SHALL return `Some` with the Groq preset

#### Scenario: Unknown name
- **WHEN** `OpenAiConfigPreset::by_name("unknown")` is called
- **THEN** it SHALL return `None`

#### Scenario: ALL contains all presets
- **WHEN** `OpenAiConfigPreset::ALL` is accessed
- **THEN** it SHALL contain at least 9 entries covering the built-in providers
