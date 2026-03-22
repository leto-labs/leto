## ADDED Requirements

### Requirement: Agent Config Supports ATIF Emission

The shared `AgentConfig` surface SHALL allow callers to enable native ATIF emission and select the target ATIF schema version for exported trajectories.

#### Scenario: Runtime receives ATIF export settings through AgentConfig

- **WHEN** a caller configures ATIF emission in `AgentConfig`
- **THEN** the effective runtime config SHALL expose both the emit toggle and
  the target schema version

### Requirement: Turn Event Surface Includes ATIF Completion Records

The shared turn event surface SHALL include ATIF completion records that can be consumed by runtime clients and export sinks.

#### Scenario: Caller receives ATIF completion records on the turn stream

- **WHEN** a turn runs with ATIF emission enabled
- **THEN** the turn stream SHALL allow the caller to observe started, step,
  final-metrics, and completed ATIF records

### Requirement: Shared Store Types Include Trajectory Persistence

The shared store-type surface SHALL include trajectory store interfaces and trajectory store lifecycle events for session-scoped ATIF persistence.

#### Scenario: Caller uses trajectory store types from brain-types

- **WHEN** a caller imports shared store traits and events from `brain-types`
- **THEN** it SHALL be able to access trajectory store interfaces and trajectory
  store event types from that crate
