## ADDED Requirements
### Requirement: Runtime Supports Session Loop Mutation

The shared `BrainRuntime` surface SHALL provide a helper for updating the
session loop override.

#### Scenario: Runtime helper updates the session loop override
- **WHEN** a caller invokes `set_session_loop(session_id, loop_name)` with a registered loop
- **THEN** the runtime SHALL persist that loop override on the session

#### Scenario: Runtime helper rejects unknown loops
- **WHEN** a caller invokes `set_session_loop(session_id, loop_name)` with an unknown loop
- **THEN** the runtime SHALL return an error
