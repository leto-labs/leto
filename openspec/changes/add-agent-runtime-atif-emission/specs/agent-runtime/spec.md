## ADDED Requirements

### Requirement: Agent Runtime Emits Native ATIF Lifecycle Events

The `agent-runtime` crate SHALL provide optional native ATIF emission as part
of the runtime event stream.

This SHALL include:

- a runtime configuration flag for enabling ATIF emission
- a trajectory-start event emitted at turn start
- completed-step events emitted for newly materialized ATIF steps
- final-metrics and completed-trajectory events emitted before turn completion

#### Scenario: Runtime emits ATIF records for an enabled turn

- **WHEN** a caller runs a turn with ATIF emission enabled
- **THEN** `agent-runtime` SHALL emit ATIF trajectory-start metadata
- **AND** SHALL emit step-completed records for the new steps from that turn
- **AND** SHALL emit final metrics and the completed trajectory before
  `TurnFinished`
