# Design: Standalone local provider crates

## Summary
The change adds `provider-llamacpp` and `provider-mistralrs` as dedicated crates that mirror the current local-provider behavior while exposing the shared `provider::Provider` trait.

The migration is additive:
- new standalone crates are added
- existing `brain-providers` modules remain unchanged
- no code-sharing dependency is introduced from `brain-*` into the new crates

## Key decisions
### Keep legacy crates intact
The new crates do not replace or wrap `brain-providers` in this change. This avoids mixing migration work with downstream integration changes.

### Heavy local backends stay opt-in
Each new crate uses `default = []` and gates its backend implementation behind a same-name feature:
- `provider-mistralrs` -> `mistralrs`
- `provider-llamacpp` -> `llamacpp`

This matches the repo's current approach for expensive local backends and keeps `cargo test --workspace` manageable.

### Shared-provider capability model
The shared SDK already separates:
- provider-wide capabilities in `ProviderInfo.capabilities`
- model-specific overrides in `ModelInfo.capabilities`

The new crates use a conservative provider baseline and allow richer per-model metadata when configured or known.

Built-in local presets should be modeled as curated shared model catalog entries rather than thin loader aliases. Each crate-specific preset owns:
- `model_info: ModelInfo`
- a backend-specific artifact/runtime payload used to build local configs

This keeps `ModelInfo` as the single provider-facing metadata type while isolating Hugging Face-facing details such as `repo_id`, exact GGUF filenames, optional `mmproj` filenames, optional revision pins, and local runtime defaults inside backend-specific structs.

The preset metadata is curated conservatively from upstream Hub metadata:
- built-in preset ids remain stable even if the artifact publisher changes
- shared `ModelInfo` reflects the configured preset variant, not the broadest possible family capability
- llama.cpp presets that ship an `mmproj` artifact are multimodal by default, and text-only behavior is an explicit opt-out
- fields that cannot be justified from Hub data, such as concrete reasoning effort labels or exact max output tokens, remain unset rather than guessed

### Runtime support policy
The migration preserves current text inference and also wires through backend-supported request shapes where feasible:
- mistral.rs: text and reasoning on the built-in presets, with additional adapter request mapping only where the backend path is both implemented and intentionally exposed
- llama.cpp: text, reasoning-aware prompt formatting, and model/config-gated multimodal input on the built-in presets, with tool-aware prompt formatting kept as an internal adapter path until it is verified end-to-end

Unsupported combinations fail explicitly with `provider::Error::Unsupported`.

### Local smoke tests
End-to-end model tests remain in-repo but are `#[ignore]` and feature-gated because they may download GGUF assets and depend on host capabilities.

The initial smoke matrix is intentionally smaller than the full theoretical adapter surface:
- llama.cpp built-in presets currently smoke-verify text, multimodal image input, and reasoning
- mistral.rs built-in presets currently smoke-verify streamed output and reasoning
- built-in preset metadata is kept aligned with what those shipped presets actually advertise and what the local smoke suite validates end-to-end
