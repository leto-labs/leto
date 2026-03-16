# Change: Add provider presets for OpenAI-compatible endpoints

## Why
`OpenAiConfig::new()` hardcodes OpenAI's base URL and model. Every OpenAI-compatible provider (Groq, DeepSeek, Together, xAI, Fireworks, Mistral, OpenRouter, Ollama) requires manual base_url and model configuration. This is tedious and error-prone. Many providers share the same `/v1/chat/completions` wire format and only differ in URL and default model.

## What Changes
- `brain-providers`: new `OpenAiConfigPreset` struct with `const` presets for 9 known providers, `ALL` list, and `by_name()` lookup
- `brain-providers`: `OpenAiConfig::new()` defaults sourced from `OpenAiConfigPreset::OPENAI`
- `cli-echo`: uses `PROVIDER` env var to select a preset by name

## Impact
- Affected specs: openai-provider (new requirement for presets)
- Affected code: brain-providers (new presets.rs, minor change to openai.rs), cli-echo (main.rs)
- No breaking changes — `OpenAiConfig::new(key)` still works identically
