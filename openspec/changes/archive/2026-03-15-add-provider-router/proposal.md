# Change: Add ProviderRouter

## Why
`OpenAiProvider` supports switching models per-request via `InferenceConfig.model`, but `MistralRsProvider` is locked to a single model loaded at construction. More broadly, there is no way to route a request to different provider backends (e.g. OpenAI for GPT-4o, MistralRs for Qwen local) based on the model name. A generic routing layer solves both problems without changing any existing provider.

## What Changes
- `brain-core`: new `ProviderRouter` struct that implements `Provider` by dispatching to registered `Arc<dyn Provider>` instances based on `InferenceConfig.model`
- No changes to `Provider` trait, `Brain`, or any existing provider implementation

## Impact
- Affected specs: brain-engine (ADDED: ProviderRouter)
- No breaking changes -- ProviderRouter is additive and optional
