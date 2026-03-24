# Design: Expand shared provider model metadata and add generated OpenAI presets

## Summary

This change separates two concerns that were previously mixed together:

- normalized runtime capability metadata for the shared provider SDK
- rich model catalog metadata used by preset systems and model selection

`provider::ProviderCapabilities` remains the runtime capability layer.
`provider::ModelInfo` becomes the shared catalog layer. `provider-openai`
regains a typed preset system built on top of that shared catalog layer.

## Shared model catalog

The shared `provider` crate will own the rich model metadata types needed by
provider plugins and preset catalogs:

- `ModelInfo`
- `ModelCost`
- `ModelLimit`

The new `ModelInfo` remains owned-data and provider-neutral, but carries most
of the data currently preserved by the legacy `brain_types::ModelInfo`:

- id and display name
- family
- reasoning effort levels
- tool-calling / structured-output / temperature / open-weights support
- knowledge cutoff and release/update dates
- input/output modalities
- pricing and limits
- lifecycle status
- optional model-specific capability overrides

This keeps `provider` reusable while preserving the static model data needed for
catalog-driven UX and configuration.

## Provider info and default model resolution

`provider::ProviderInfo` will keep the full model catalog and replace the
current `default_model: Option<String>` field with a default model id/reference
field.

The shared crate will provide a small helper that resolves the default model id
into the corresponding `ModelInfo` when the model is present in the catalog.

This avoids duplicating the full model object while still making the default
selection explicit.

## OpenAI preset system

`provider-openai` regains a typed preset module that mirrors the legacy
OpenAI-compatible preset ergonomics:

- typed preset constants
- provider/display name
- base URL
- env key
- default model id
- model catalog
- supported API surfaces

The preset module remains provider-native and does not move into the shared
`provider` crate.

## Config ownership

`provider-openai::Config` will expand so configs constructed from presets retain
the metadata needed by the plugin implementation:

- provider name
- base URL
- default model id
- model catalog
- supported API surfaces
- API surface mode

Library code will not read env vars. Presets expose `env_key`; tests and
applications decide whether to read that variable.

## Generator migration

The existing `tools/provider-preset-gen` tool will be updated instead of
creating a second generator.

Changes:

- output path changes from `crates/brain-providers/src/openai/presets` to
  `crates/provider-openai/src/presets`
- generated code references `provider::{ModelCost, ModelInfo, ModelLimit}`
  instead of `brain_types`
- generated preset/module types reference `provider-openai` config and API
  surface types

The generator continues to source data from the local `models.dev` checkout in
`repocache/anomalyco/models.dev`, and the generated Rust files remain checked
in and verifiable with `--check`.

## Testing

This change requires:

- shared model metadata serde tests
- preset lookup/config construction tests in `provider-openai`
- generator tests and `--check`
- env-gated smoke tests across the full OpenAI-compatible preset matrix,
  skipping cleanly when a preset's declared env var is absent
