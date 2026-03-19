# Change: Generate OpenAI-compatible preset Rust from models.dev

## Why

`crates/brain-providers/src/openai/presets.rs` was a large handwritten source of truth for both provider metadata and model catalogs. That made it easy for the preset layer to drift from real provider model surfaces and hard to refresh when providers added new models.

We already use models.dev as an external model catalog reference. This change makes that relationship explicit by generating the Rust preset modules from a managed local models.dev checkout, while keeping runtime provider code handwritten.

## What Changes

- Add a Rust workspace generator, `provider-preset-gen`
- Replace the handwritten flat `openai/presets.rs` file with generated per-provider preset modules under `openai/presets/`
- Source model metadata from `repocache/anomalyco/models.dev`, cloning that repo on demand if it is missing
- Keep a generator-side allowlist and per-provider override table so only supported OpenAI-compatible API presets are emitted
- Filter non-chat models out of generated chat preset catalogs

## Impact

- Affected spec: `brain-providers-openai-api`
- Affected code: `brain-providers`, `tools/provider-preset-gen`
- Developer workflow gains a deterministic way to refresh preset Rust without hand-editing model arrays
