# brain-types Specification Delta

## MODIFIED Requirements

### Requirement: Model Helper Methods Live On Runtime
The system SHALL allow `BrainRuntime` to expose model-oriented helper methods
because those depend on provider registry state rather than raw store CRUD.

These helper methods SHALL include:

- list available models
- get the current model for a session
- set a session model by model ID
- get the effective thought level for a session
- set a session thought level by value

#### Scenario: Plain CRUD remains on store
- **WHEN** a caller needs session or message CRUD
- **THEN** it SHALL use `runtime.store()`
- **AND** the runtime trait SHALL not mirror those methods

#### Scenario: Model helper resolves and persists model selection
- **WHEN** a caller sets a session model by model ID
- **THEN** the runtime SHALL resolve that model to a unique provider
- **AND** persist the resulting provider/model inference pair on the session

#### Scenario: Runtime persists session thought level override
- **WHEN** a caller sets a session thought level by value
- **THEN** the runtime SHALL persist that value in the session inference override layer
- **AND** the value SHALL participate in effective session inference resolution
