## 1. Shared provider catalog

- [x] Expand `provider::ModelInfo` to preserve rich catalog metadata
- [x] Add shared `ModelCost` and `ModelLimit` types
- [x] Update `ProviderInfo` default-model metadata and helpers
- [x] Update mock provider/tests to use the richer shared catalog types

## 2. OpenAI preset system

- [x] Add typed preset modules under `crates/provider-openai/src/presets`
- [x] Expand `provider-openai::Config` to support preset-backed metadata
- [x] Update `OpenAiProvider::info()` to return preset-backed model catalogs
- [x] Add preset lookup/config tests

## 3. Generator and verification

- [x] Retarget `tools/provider-preset-gen` to `provider-openai`
- [x] Add or update generator tests and `--check` behavior
- [x] Add env-gated smoke tests that iterate the OpenAI-compatible preset matrix
- [x] Validate and archive the OpenSpec change
