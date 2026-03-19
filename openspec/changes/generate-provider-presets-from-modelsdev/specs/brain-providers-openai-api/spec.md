## ADDED Requirements

### Requirement: Generated preset modules
The system SHALL provide a workspace Rust generator that emits the checked-in `OpenAiConfigPreset` Rust modules for supported OpenAI-compatible API providers from a managed local models.dev checkout. The generated output SHALL remain compatible with the public `OpenAiConfigPreset` API used by callers.

#### Scenario: Generate preset modules from models.dev
- **WHEN** the preset generator is run
- **THEN** it SHALL read the configured provider allowlist from the local models.dev checkout
- **AND** it SHALL write per-provider preset Rust modules under `crates/brain-providers/src/openai/presets/`
- **AND** those generated modules SHALL expose the same built-in preset constants through `OpenAiConfigPreset`

#### Scenario: Verify generated preset modules are current
- **WHEN** the preset generator is run in check mode
- **THEN** it SHALL fail if any checked-in generated preset module differs from the current generated output

### Requirement: Chat preset catalogs
The system SHALL generate model catalogs for OpenAI-compatible chat presets from the supported models.dev providers while filtering models that are not suitable for the chat-completions surface exposed by `OpenAiProvider`.

#### Scenario: Non-chat models are excluded
- **WHEN** a source provider includes embedding or other non-chat models
- **THEN** the generated preset catalog SHALL exclude those models from `OpenAiConfigPreset.models`

#### Scenario: Recent supported chat models are included
- **WHEN** the source provider includes newer supported chat models
- **THEN** the generated preset catalog SHALL include those model IDs in the corresponding preset
