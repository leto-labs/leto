# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Re-export Facade Updated
`brain-core` SHALL continue to re-export all sub-crates (`brain-types`, `brain-providers`, `brain-stores`, `brain-loops`, `brain-tools`, `brain-transports`). The `examples/cli-echo` and `examples/cli-local` binaries have been removed from the workspace — their functionality is subsumed by `brain-cli`.

#### Scenario: Workspace members updated
- **WHEN** the workspace is built
- **THEN** `brain-cli` SHALL be the user-facing binary crate
- **AND** `examples/cli-echo` and `examples/cli-local` SHALL NOT be present

