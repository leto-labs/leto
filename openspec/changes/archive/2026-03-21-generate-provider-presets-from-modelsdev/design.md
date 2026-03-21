# Design: Generate OpenAI-compatible preset Rust from models.dev

## Context

The preset layer currently bundles three concerns into one handwritten file:

1. Provider selection metadata such as `name`, `env_key`, `base_url`, and `default_model`
2. Provider model catalogs as `ModelInfo` arrays
3. Public preset assembly through `OpenAiConfigPreset`

That structure is difficult to maintain because model catalogs change more frequently than the preset API itself.

## Decision

Introduce a workspace generator that emits the Rust preset modules themselves.

The generator:

- owns a provider allowlist for supported OpenAI-compatible API presets
- manages a single local source checkout at `repocache/anomalyco/models.dev`
- parses models.dev TOML files for the allowed providers
- applies a small amount of local policy, including:
  - provider-level overrides for `base_url`, `env_key`, and `default_model`
  - filtering deprecated or non-chat models
  - excluding OpenAI Codex models from the normal API preset path
- writes checked-in Rust files to `crates/brain-providers/src/openai/presets/`

## Consequences

### Benefits

- model catalog refreshes stop requiring large handwritten Rust edits
- the public `OpenAiConfigPreset` API stays stable
- model discovery used by ACP and CLI surfaces picks up newer chat models more quickly
- the supported provider boundary remains explicit through the allowlist

### Trade-offs

- generation now depends on a maintained local checkout of models.dev
- some preset metadata remains locally overridden because models.dev does not define runtime wiring the way `brain` needs it
- the generator includes light filtering policy, so the generated output is not a raw mirror of upstream data

## Rejected Alternatives

### Keep a vendored snapshot in this repo

Rejected because the user explicitly prefers a single source path backed by the existing `repocache` workflow.

### Generate only raw model arrays

Rejected because the goal is to autogenerate the Rust preset modules themselves, not just a lower-level catalog fragment.
