# Change: Update provider presets — env_key, from_env, Gemini/MiniMax, smoke tests

## Why
Presets lacked a way to auto-discover API keys from environment variables. Users had to manually match env var names to presets. Also missing presets for Gemini and MiniMax, and no integration tests to verify providers work.

## What Changes
- `OpenAiConfigPreset`: added `env_key` field and `from_env()` method
- New presets: Gemini, MiniMax (11 total, up from 9)
- Updated `Built-in presets` requirement: now 11 providers, includes `env_key`
- New integration test suite: `tests/provider_smoke.rs` with per-provider smoke tests
- `cli-echo`: simplified to use `preset.from_env()` for automatic key discovery

## Impact
- Affected specs: openai-provider (modified requirements)
- Affected code: brain-providers (presets.rs, new test file), cli-echo (main.rs)
- No breaking changes
