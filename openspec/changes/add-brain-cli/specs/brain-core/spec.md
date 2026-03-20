# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Re-export Facade Updated
`brain-core` SHALL continue to re-export the runtime-facing crates used by the
local CLI path. The old `cli-echo` and `cli-local` binaries have been removed
from the workspace because their behavior is subsumed by `brain-cli`.

#### Scenario: Workspace members updated
- **WHEN** the focused runtime workspace is built
- **THEN** `brain-cli` SHALL be the user-facing local binary crate
- **AND** `examples/cli-echo` and `examples/cli-local` SHALL NOT be present
