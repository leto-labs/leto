# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: ProviderRouter
The system SHALL provide a `ProviderRouter` struct in `brain-core` that implements the `Provider` trait. It SHALL hold a map of model names to `Arc<dyn Provider>` instances and a default provider. When `chat()` is called, it SHALL read `InferenceConfig.model` to look up the target provider, falling back to the default if the model is `None` or not found in the map.

#### Scenario: Route by model name
- **WHEN** `chat()` is called with `InferenceConfig { model: Some("gpt-4o") }`
- **AND** a provider is registered under the name `"gpt-4o"`
- **THEN** the call SHALL be dispatched to that provider

#### Scenario: Fallback to default
- **WHEN** `chat()` is called with `InferenceConfig { model: None }`
- **THEN** the call SHALL be dispatched to the default provider

#### Scenario: Unknown model falls back
- **WHEN** `chat()` is called with a model name not registered in the router
- **THEN** the call SHALL be dispatched to the default provider

#### Scenario: Construction
- **WHEN** `ProviderRouter::new(default)` is called and providers are added via `add(name, provider)`
- **THEN** the router SHALL be usable as an `Arc<dyn Provider>` passed to `Brain::new()`
