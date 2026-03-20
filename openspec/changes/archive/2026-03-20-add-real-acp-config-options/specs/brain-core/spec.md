# brain-core Specification Delta

## ADDED Requirements

### Requirement: Session Inference Supports Thought Level Overrides

The core runtime SHALL treat thought level as part of session-scoped inference.

The core SHALL:

- merge project default thought level with session overrides
- validate thought-level updates against the current effective model's supported reasoning levels
- omit the effective thought level when the current model has no reasoning capability

#### Scenario: Effective thought level merges from project and session
- **WHEN** a project has a default thought level and a session sets a different thought level
- **THEN** the effective session inference SHALL use the session value

#### Scenario: Invalid thought level is rejected
- **WHEN** a caller attempts to set a thought level unsupported by the current model
- **THEN** the runtime SHALL reject the update

#### Scenario: Non-reasoning model clears effective thought level
- **WHEN** the effective session model does not advertise reasoning support
- **THEN** the effective session inference SHALL have no thought-level value
