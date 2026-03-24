## MODIFIED Requirements

### Requirement: Provider-OpenAI Exposes A Shared Provider Implementation

The `provider-openai` crate SHALL expose a provider implementation for the
shared `provider::Provider` trait while preserving its protocol-native client.

#### Scenario: OpenAI acts as a provider plugin

- **WHEN** a caller constructs the shared OpenAI provider implementation
- **THEN** it SHALL accept shared provider requests
- **AND** stream shared provider events translated from the API surface selected
  by the resolved OpenAI-compatible configuration

### Requirement: Provider-OpenAI Smoke Tests Cover The Built-In Preset Matrix

The `provider-openai` crate SHALL include env-gated smoke coverage across the
built-in OpenAI-compatible preset matrix.

#### Scenario: Smoke tests skip presets without credentials

- **WHEN** a preset smoke test runs and the preset's declared env var is absent
- **THEN** the test SHALL skip cleanly for that preset

#### Scenario: Smoke tests use the best supported API surface

- **WHEN** a preset smoke test runs for a preset with credentials
- **THEN** it SHALL issue a basic inference request against a supported API
  surface for that preset

## ADDED Requirements

### Requirement: Shared OpenAI Provider Supports Every Declared API Surface

The shared `OpenAiProvider` implementation SHALL support every API surface
declared by its resolved `provider-openai::Config`.

#### Scenario: Responses-backed provider stream

- **WHEN** the resolved surface is `Responses`
- **THEN** the shared adapter SHALL stream shared provider events translated
  from the Responses API

#### Scenario: Chat-completions-backed provider stream

- **WHEN** the resolved surface is `ChatCompletions`
- **THEN** the shared adapter SHALL stream shared provider events translated
  from Chat Completions streaming chunks

#### Scenario: Unsupported forced surface errors cleanly

- **WHEN** a config forces an API surface that the preset/config does not
  support
- **THEN** the shared adapter SHALL return a configuration/runtime error rather
  than silently falling back

### Requirement: Shared OpenAI Provider Capabilities Reflect The Active Surface

The shared OpenAI provider metadata SHALL advertise capabilities for the active
resolved surface rather than the union across all possible surfaces.

#### Scenario: Forced chat-completions config narrows capabilities

- **WHEN** a config resolves to `ChatCompletions`
- **THEN** `OpenAiProvider::info()` SHALL report the capability set supported by
  Chat Completions

#### Scenario: Forced responses config advertises responses capabilities

- **WHEN** a config resolves to `Responses`
- **THEN** `OpenAiProvider::info()` SHALL report the capability set supported by
  Responses

### Requirement: Smoke Tests Cover Every Supported Preset-Surface Combination

Wire-client and shared-provider smoke coverage SHALL exercise every declared
supported API surface for each built-in preset with available credentials.

#### Scenario: Wire smoke iterates all declared surfaces

- **WHEN** a preset advertises more than one supported API surface
- **THEN** the wire smoke suite SHALL test each declared surface explicitly

#### Scenario: Shared adapter smoke iterates all declared surfaces

- **WHEN** a preset advertises more than one supported API surface
- **THEN** the shared adapter smoke suite SHALL test each declared surface
  explicitly

#### Scenario: Declared incompatible surface fails the smoke suite

- **WHEN** a configured preset/surface combination fails due to parser or
  compatibility drift
- **THEN** the smoke suite SHALL fail for that combination instead of treating
  it as an optional skip
