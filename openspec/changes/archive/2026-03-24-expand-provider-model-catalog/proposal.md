# Change: Expand shared provider model metadata and add generated OpenAI presets

## Why

The new `provider` SDK has a better normalized inference contract than the
legacy `brain-*` stack, but it lost most of the model catalog richness that
powered the legacy OpenAI preset matrix. In particular:

- shared `provider::ModelInfo` is currently too small to preserve pricing,
  limits, modalities, reasoning levels, and lifecycle metadata
- `provider-openai` does not yet expose generated typed presets or a preset-
  backed model catalog
- the existing `provider-preset-gen` tool still generates `brain-providers`
  output and depends on `brain_types::ModelInfo`

We want the `provider-*` crates to stand on their own as the production-grade
provider SDK, without depending on any `brain-*` crate, while preserving the
useful model metadata and preset ergonomics from the legacy system.

## What Changes

- Expand shared `provider::ModelInfo` to carry rich model catalog metadata
- Add shared `provider::ModelCost` and `provider::ModelLimit` types
- Change shared `provider::ProviderInfo` to expose a default model id/reference
  plus a rich model catalog
- Add generated typed presets to `provider-openai`
- Expand `provider-openai::Config` so preset-backed configs can carry provider
  identity, model catalog metadata, and supported API surfaces
- Retarget `tools/provider-preset-gen` to generate `provider-openai` presets
  using the shared `provider` model types
- Add env-gated smoke tests that iterate the OpenAI-compatible preset matrix

## Impact

- Affected specs: `provider`, `provider-openai`
- Affected crates: `provider`, `provider-openai`
- Affected tool: `tools/provider-preset-gen`
