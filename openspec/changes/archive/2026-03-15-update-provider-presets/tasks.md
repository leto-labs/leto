## 1. Implementation
- [x] 1.1 Add `env_key` field and `from_env()` method to `OpenAiConfigPreset`
- [x] 1.2 Add Gemini and MiniMax presets (11 total)
- [x] 1.3 Create `tests/provider_smoke.rs` with per-provider smoke tests
- [x] 1.4 Update `cli-echo` to use `preset.from_env()` for automatic key discovery

## 2. Verify
- [x] 2.1 `cargo build --workspace` succeeds
- [x] 2.2 `cargo test --workspace` passes
- [x] 2.3 Smoke tests run against available providers
