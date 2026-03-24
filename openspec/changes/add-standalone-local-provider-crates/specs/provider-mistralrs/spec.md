# provider-mistralrs Specification

## ADDED Requirements

### Requirement: Provider-MistralRs Owns A Standalone Local Inference Crate
The system SHALL provide a standalone `provider-mistralrs` crate for mistral.rs-backed local inference.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Standalone crate exposes local provider types
- **WHEN** a caller depends on `provider-mistralrs`
- **THEN** it SHALL be able to construct `MistralRsConfig`, `MistralRsModelPreset`, and `MistralRsProvider` without importing `brain-types`

### Requirement: Provider-MistralRs Preserves Local Model Registry Behavior
The `provider-mistralrs` crate SHALL preserve named model registration, lazy loading, caching, and explicit preload.

#### Scenario: Lazy load and preload
- **WHEN** a caller constructs a provider with multiple named configs
- **THEN** no model SHALL be loaded at construction time
- **AND** `preload(name)` SHALL eagerly resolve and cache the selected model

### Requirement: Provider-MistralRs Presets Expose Shared Model Metadata
Built-in mistral.rs presets SHALL embed shared `ModelInfo` metadata alongside local artifact details.

#### Scenario: Preset-derived config carries model catalog metadata
- **WHEN** a caller converts a built-in `MistralRsModelPreset` into config
- **THEN** the resulting config SHALL carry the preset's `ModelInfo`
- **AND** local artifact data such as `repo_id`, exact GGUF filename lists, and optional revision pins SHALL remain backend-specific

#### Scenario: Stable preset id can outlive artifact publisher changes
- **WHEN** a built-in preset needs a different GGUF publisher to preserve the intended quantized artifact
- **THEN** the shared model id exposed by the preset SHALL remain stable
- **AND** only the backend-specific artifact coordinates SHALL change

### Requirement: Provider-MistralRs Implements The Shared Provider Trait
The `provider-mistralrs` crate SHALL implement `provider::Provider`.

#### Scenario: Shared provider streams local inference events
- **WHEN** a caller sends a shared request
- **THEN** the provider SHALL stream shared response, block, usage, and completion events

### Requirement: Provider-MistralRs Supports Backend-Mapped Local Features
The `provider-mistralrs` provider SHALL map backend-supported request features from the shared provider surface.

#### Scenario: Built-in presets advertise the smoke-verified capability set conservatively
- **WHEN** a caller uses one of the built-in `MistralRsModelPreset` values
- **THEN** the preset metadata SHALL advertise only the capabilities that the shipped preset variant intentionally exposes
- **AND** the initial built-in presets SHALL remain text-only for input modalities
- **AND** the initial built-in presets SHALL advertise reasoning support without claiming tool-call support by default

#### Scenario: Model-gated vision input
- **WHEN** a request contains image input for a selected vision-capable model
- **THEN** the provider SHALL translate the image content into mistral.rs vision messages
- **AND** a non-vision model SHALL fail explicitly

#### Scenario: Reasoning toggle
- **WHEN** a request asks for reasoning behavior
- **THEN** the provider SHALL map that request into the available mistral.rs thinking controls

### Requirement: Provider-MistralRs Is Feature Gated
The `provider-mistralrs` crate SHALL gate backend-heavy APIs behind the `mistralrs` feature with no default features enabled.

#### Scenario: Feature disabled
- **WHEN** `provider-mistralrs` is compiled without the `mistralrs` feature
- **THEN** backend-heavy types SHALL not be available
