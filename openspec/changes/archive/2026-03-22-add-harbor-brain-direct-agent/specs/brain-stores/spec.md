## ADDED Requirements

### Requirement: Store Exposes Session Trajectory Persistence

The composed `Store` surface SHALL expose session-scoped ATIF trajectory persistence through a dedicated trajectory sub-store.

#### Scenario: Runtime loads existing trajectory state for a session

- **WHEN** a caller has `&dyn Store`
- **THEN** it SHALL be able to access a trajectory sub-store
- **AND** load the persisted ATIF trajectory for a session when one exists

### Requirement: Trajectories Persist Separately From Messages

ATIF trajectory persistence SHALL remain separate from message persistence so historical transcript metadata can be preserved across turns.

#### Scenario: Trajectory state is updated without replaying old messages

- **WHEN** a later turn appends new transcript steps for a session
- **THEN** the runtime SHALL update the persisted ATIF trajectory state
- **AND** it SHALL NOT need to restamp historical ATIF step metadata from the
  latest config
