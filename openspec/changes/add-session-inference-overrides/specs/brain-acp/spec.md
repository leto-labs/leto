# brain-acp Specification

## ADDED Requirements

### Requirement: Real ACP Session Model State

When ACP unstable session model support is enabled, the real ACP backend SHALL
report model state from the real `brain-core` session/runtime model rather than
from ACP-local adapter state.

#### Scenario: New session reports effective model state

- **WHEN** a client creates a real ACP session
- **THEN** the response SHALL include the current effective model derived from the session and project config
- **AND** it SHALL include the available unambiguous models exposed by the provider router

#### Scenario: Load session reports persisted model state

- **WHEN** a client loads a real ACP session
- **THEN** the response SHALL include the current persisted session model state

### Requirement: Real ACP Session Model Switching

When ACP unstable session model support is enabled, the real ACP backend SHALL
support `session/set_model` through persisted session inference overrides.

#### Scenario: session/set_model persists provider and model on the session

- **WHEN** ACP `session/set_model` is called with a valid model ID
- **THEN** the real ACP backend SHALL resolve the owning provider through `ProviderRouter`
- **AND** it SHALL persist provider+model on the real `Session.inference`

#### Scenario: session/set_model preserves other session inference fields

- **WHEN** a session already has per-session `temperature` or `max_tokens` overrides
- **THEN** changing only the model through ACP SHALL preserve those other session-local overrides
