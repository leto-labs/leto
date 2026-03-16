# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## MODIFIED Requirements

### Requirement: AgentConfig
The agent loop SHALL accept an `AgentConfig` with at minimum: `max_iterations` (default 20), `system_prompt` (optional String), and `inference: InferenceConfig`. The `InferenceConfig` carries `model: Option<String>`, `max_tokens: Option<u32>`, and `temperature: Option<f32>`. When `model` is `None`, the provider uses its own default.

#### Scenario: Default config
- **WHEN** AgentConfig::default() is used
- **THEN** max_iterations SHALL be 20, system_prompt SHALL be None, and inference.model SHALL be None

#### Scenario: Config propagates to provider
- **WHEN** `AgentConfig { inference: InferenceConfig { model: Some("gpt-4o"), .. }, .. }` is used
- **THEN** the agent loop SHALL pass this inference config to the provider's `chat()` method
