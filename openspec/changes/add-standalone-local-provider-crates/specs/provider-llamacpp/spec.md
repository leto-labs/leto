# provider-llamacpp Specification

## ADDED Requirements

### Requirement: Provider-LlamaCpp Owns A Standalone Local Inference Crate
The system SHALL provide a standalone `provider-llamacpp` crate for llama.cpp-backed local inference.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Standalone crate exposes local provider types
- **WHEN** a caller depends on `provider-llamacpp`
- **THEN** it SHALL be able to construct `LlamaCppConfig`, `LlamaCppModelPreset`, and `LlamaCppProvider` without importing `brain-types`

### Requirement: Provider-LlamaCpp Preserves Local Model Registry Behavior
The `provider-llamacpp` crate SHALL preserve named model registration, lazy loading, GGUF resolution, caching, and explicit preload.

#### Scenario: Lazy load and preload
- **WHEN** a caller constructs a provider with multiple named configs
- **THEN** no model SHALL be loaded at construction time
- **AND** `preload(name)` SHALL eagerly resolve and cache the selected model

### Requirement: Provider-LlamaCpp Presets Expose Shared Model Metadata
Built-in llama.cpp presets SHALL embed shared `ModelInfo` metadata alongside local artifact details.

#### Scenario: Preset-derived config carries model catalog metadata
- **WHEN** a caller converts a built-in `LlamaCppModelPreset` into config
- **THEN** the resulting config SHALL carry the preset's `ModelInfo`
- **AND** local artifact data such as `repo_id`, exact GGUF filenames, optional `mmproj` filenames, and optional revision pins SHALL remain backend-specific

#### Scenario: Presets with multimodal artifacts are multimodal by default
- **WHEN** a caller converts a built-in `LlamaCppModelPreset` using the default config path
- **AND** the preset carries an `mmproj` filename
- **THEN** the resulting config SHALL attach that `mmproj` filename by default
- **AND** the resulting config metadata SHALL advertise image input by default

### Requirement: Provider-LlamaCpp Implements The Shared Provider Trait
The `provider-llamacpp` crate SHALL implement `provider::Provider`.

#### Scenario: Shared provider streams local inference events
- **WHEN** a caller sends a shared request
- **THEN** the provider SHALL stream shared response, block, usage, and completion events

### Requirement: Provider-LlamaCpp Supports Backend-Mapped Local Features
The `provider-llamacpp` provider SHALL map backend-supported request features from the shared provider surface.

#### Scenario: Built-in presets advertise the smoke-verified capability set conservatively
- **WHEN** a caller uses one of the built-in `LlamaCppModelPreset` values
- **THEN** the preset metadata SHALL advertise multimodal input for presets that ship an `mmproj` artifact
- **AND** the initial built-in presets SHALL advertise reasoning support
- **AND** the initial built-in presets SHALL NOT claim tool-call support until that path is verified end-to-end for the shipped preset variants

#### Scenario: Model/config-gated multimodal input
- **WHEN** a request includes image input and the selected model/config supports multimodal evaluation
- **THEN** the provider SHALL use llama.cpp's multimodal path
- **AND** an unsupported config SHALL fail explicitly

#### Scenario: Reasoning-aware prompt formatting
- **WHEN** a request asks for reasoning behavior
- **THEN** the provider SHALL map that request into the available llama.cpp template controls

### Requirement: Provider-LlamaCpp Is Feature Gated
The `provider-llamacpp` crate SHALL gate backend-heavy APIs behind the `llamacpp` feature with no default features enabled.

#### Scenario: Feature disabled
- **WHEN** `provider-llamacpp` is compiled without the `llamacpp` feature
- **THEN** backend-heavy types SHALL not be available
