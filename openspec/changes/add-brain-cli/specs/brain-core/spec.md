# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Re-export Facade Preserved
The migration requested by this change SHALL remain pending until implementation is completed.
`examples/cli-echo` and `examples/cli-local` SHALL remain workspace members in this worktree.
All `brain-core` re-exports SHALL remain unchanged.

#### Scenario: Workspace members updated
- **WHEN** the workspace is built
- **THEN** it SHALL still include `examples/cli-echo` and `examples/cli-local`



