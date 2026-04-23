## MODIFIED Requirements
### Requirement: Generated preset modules

The system SHALL provide a Rust generator in the vendored `ai-provider-rs`
workspace that emits the checked-in `OpenAiConfigPreset` Rust modules for
supported OpenAI-compatible API providers from a managed local models.dev
checkout. The generated output SHALL remain compatible with the public
`OpenAiConfigPreset` API used by callers.

#### Scenario: Generate preset modules from models.dev
- **WHEN** the preset generator is run
- **THEN** it SHALL read the configured provider allowlist from the local
  models.dev checkout
- **AND** it SHALL write per-provider preset Rust modules under
  `submodules/ai-provider-rs/crates/provider-openai/src/presets/`
- **AND** those generated modules SHALL expose the same built-in preset
  constants through `OpenAiConfigPreset`

#### Scenario: Verify generated preset modules are current
- **WHEN** the preset generator is run in check mode
- **THEN** it SHALL fail if any checked-in generated preset module differs from
  the current generated output
