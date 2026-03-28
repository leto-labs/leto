# brain-providers-llamacpp Specification

## Purpose
In-process GGUF inference using llama.cpp as a `Provider` implementation in `brain-providers`.

## Requirements

### Requirement: LlamaCppConfig
The system SHALL define a `LlamaCppConfig` struct with:
- `model_id` (`String`) — HuggingFace repo ID or local directory path
- `gguf_file` (`String`) — GGUF filename inside the repo/directory
- `n_gpu_layers` (`Option<u32>`) — optional GPU layer offload count
- `context_size` (`u32`) — model context size in tokens

#### Scenario: Config from preset
- **WHEN** `LlamaCppModelPreset::QWEN35_0_8B.into_config()` is called
- **THEN** it SHALL produce a `LlamaCppConfig` with `model_id` set to the Qwen preset repo ID and matching `gguf_file`
- **AND** `n_gpu_layers` SHALL be `None` until explicitly configured

#### Scenario: Custom model path
- **WHEN** `LlamaCppConfig` is built with a local directory path and file name
- **THEN** the provider SHALL load the file directly from that directory when selected

### Requirement: LlamaCppModelPreset
The system SHALL define `LlamaCppModelPreset` with `name`, `model_id`, and `gguf_file`, plus `ALL` and `by_name()`.

#### Scenario: Preset lookup
- **WHEN** `LlamaCppModelPreset::by_name("qwen3.5-0.8b")` is called
- **THEN** it SHALL return the matching preset

#### Scenario: All presets
- **WHEN** `LlamaCppModelPreset::ALL` is accessed
- **THEN** it SHALL include at least the Qwen 3.5-0.8B and 2B presets

### Requirement: LlamaCppProvider
The system SHALL implement `Provider` for `LlamaCppProvider`. The provider SHALL:
- be constructible from registered model configs plus a default model
- route inference by `InferenceConfig.model` and fall back to default model
- lazily resolve models at first chat call
- support `preload(name)` to eagerly load and cache a model
- emit `ChatChunk::Delta` for token text and `ChatChunk::Done` with token usage

#### Scenario: Text response
- **WHEN** `chat()` is called with user messages
- **THEN** the provider SHALL stream text deltas and finish with a `ChatChunk::Done`

#### Scenario: Model selection by config
- **WHEN** `InferenceConfig.model` is `Some("qwen3.5-0.8b")` and that model is registered
- **THEN** the provider SHALL use that model
- **WHEN** `InferenceConfig.model` is `None`
- **THEN** it SHALL use the default model

#### Scenario: Lazy loading behavior
- **WHEN** `LlamaCppProvider::new` is called with multiple presets
- **THEN** no model SHALL be loaded during construction
- **AND** the first `chat()` call SHALL resolve and cache the selected model

#### Scenario: Preload
- **WHEN** `preload("qwen3.5-0.8b")` is called
- **THEN** the model SHALL be loaded immediately and reused by later chat calls

### Requirement: Feature Gate
The `LlamaCppProvider`, `LlamaCppConfig`, and `LlamaCppModelPreset` SHALL be gated behind the `llamacpp` feature of `brain-providers`.

#### Scenario: Feature disabled
- **WHEN** `brain-providers` is compiled without `llamacpp`
- **THEN** the provider and preset types SHALL not be available
