# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## MODIFIED Requirements

### Requirement: LocalConfig
The system SHALL define a `MistralRsConfig` struct with `model_id` (HuggingFace repo ID or local directory path), `gguf_files` (list of GGUF filenames), and `device` (DevicePreference enum: Auto or Cpu). This configures which model to load and how.

#### Scenario: Config from preset
- **WHEN** `MistralRsModelPreset::QWEN3_0_6B.into_config()` is called
- **THEN** the resulting `MistralRsConfig` SHALL have `model_id` pointing to the Qwen 3 0.6B GGUF repo and `device` set to `Auto`

#### Scenario: Custom model path
- **WHEN** `MistralRsConfig` is constructed with a local directory path
- **THEN** the provider SHALL load GGUF files from that directory without downloading

### Requirement: LocalProvider
The system SHALL implement the `Provider` trait for `MistralRsProvider` in the `brain-providers` crate. It SHALL use mistral.rs `GgufModelBuilder` to load GGUF models and `stream_chat_request` to stream inference results. Construction SHALL be async since model loading may involve downloading from HuggingFace and takes significant time.

#### Scenario: Text response
- **WHEN** `chat()` is called with user messages
- **THEN** the provider SHALL yield `ChatChunk::Delta` for each text fragment and `ChatChunk::Done` with usage at the end

#### Scenario: Model auto-download
- **WHEN** a HuggingFace repo ID is provided and the model is not cached
- **THEN** the provider SHALL download the GGUF files on first use and cache them locally

#### Scenario: Cached model
- **WHEN** a model has been previously downloaded
- **THEN** the provider SHALL load from the local cache without network access

### Requirement: LocalModelPreset
The system SHALL define a `MistralRsModelPreset` struct with `name`, `model_id`, and `gguf_files` fields. It SHALL provide const presets for at least Qwen 3 0.6B, 1.7B, and 4B GGUF models. An `ALL` slice and `by_name()` lookup SHALL be provided.

#### Scenario: Preset lookup
- **WHEN** `MistralRsModelPreset::by_name("qwen3-0.6b")` is called
- **THEN** it SHALL return the Qwen 3 0.6B preset

#### Scenario: ALL contains presets
- **WHEN** `MistralRsModelPreset::ALL` is accessed
- **THEN** it SHALL contain at least 3 entries

### Requirement: Feature Gate
The `MistralRsProvider` SHALL be gated behind the `mistralrs` feature flag (off by default). Building without the `mistralrs` feature SHALL not compile mistral.rs or any of its dependencies.

#### Scenario: Feature disabled
- **WHEN** `brain-providers` is compiled without the `mistralrs` feature
- **THEN** `MistralRsProvider`, `MistralRsConfig`, and `MistralRsModelPreset` SHALL not be available
