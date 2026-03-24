## MODIFIED Requirements

### Requirement: Provider-OpenAI Presets Are Generated From models.dev

The system SHALL provide a workspace Rust generator that emits the checked-in
`provider-openai` preset modules from the managed local `models.dev` checkout.

#### Scenario: Generate provider-openai presets from models.dev

- **WHEN** the generator runs
- **THEN** it SHALL read the configured provider allowlist from the local
  `models.dev` checkout
- **AND** emit the checked-in preset modules under `provider-openai`
- **AND** generate model catalogs using the shared `provider` model metadata
- **AND** format the emitted Rust source before writing it

#### Scenario: Verify generated provider-openai presets are current

- **WHEN** the generator runs with `--check`
- **THEN** it SHALL fail if any checked-in generated preset file differs from
  the current formatted generated output
