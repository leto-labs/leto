## ADDED Requirements

### Requirement: Agent CLI Exec Uses Runtime-Emitted ATIF

The `agent-cli` exec flow SHALL consume runtime-emitted ATIF records instead of
rebuilding trajectories from streamed terminal text.

#### Scenario: Exec writes validated trajectory output

- **WHEN** `agent exec` completes with ATIF emission enabled
- **THEN** the CLI SHALL assemble ATIF state from received runtime ATIF events
- **AND** validate the resulting trajectory
- **AND** write the validated trajectory to `trajectory.json`
