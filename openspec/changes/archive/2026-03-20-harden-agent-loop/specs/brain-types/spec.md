# brain-types Specification Delta

## ADDED Requirements

### Requirement: AgentConfig Supports Hardening Fields

The agent loop SHALL accept an `AgentConfig` with at minimum:

- `max_iterations` (default 20)
- `system_prompt` (optional String)
- `loop_name` (optional String)
- `doom_loop_threshold` (default 3)
- `doom_loop_strategy` (default `steer`)
- `compaction_threshold` (default `Some(0.8)`)
- `compaction_model` (optional String)
- `max_retries` (default 3)
- `inference: InferenceConfig`

When these newer hardening fields are missing during deserialization, the
system SHALL supply defaults.

#### Scenario: AgentConfig defaults include hardening settings

- **WHEN** `AgentConfig::default()` is used
- **THEN** the doom-loop, compaction, and retry settings SHALL use their
  documented defaults

#### Scenario: Older serialized configs remain readable

- **WHEN** an `AgentConfig` is deserialized without the newer hardening fields
- **THEN** deserialization SHALL succeed
- **AND** the missing fields SHALL use defaults
