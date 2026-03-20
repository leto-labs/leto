# brain-acp Specification

## Purpose
TBD - created by archiving change add-brain-runtime-trait. Update Purpose after archive.
## Requirements
### Requirement: ACP Backend Depends On BrainRuntime
The real ACP backend SHALL depend on the shared `BrainRuntime` boundary rather
than on `Brain` plus `ProviderRouter`.

The backend SHALL:

- hold a runtime trait object in app state
- use `runtime.resolve_or_create_project()` for cwd-to-project bootstrap
- use `runtime.turn()` and `runtime.cancel_turn()` for active turn lifecycle
- use `runtime` model helper methods for session-model inspection and updates
- use `runtime.store()` for plain session/message/project CRUD

#### Scenario: Real backend uses runtime for project and turn handling
- **WHEN** ACP creates or prompts a real session
- **THEN** it SHALL resolve the project through `BrainRuntime`
- **AND** execute and cancel turns through `BrainRuntime`

#### Scenario: Real backend uses runtime model helpers
- **WHEN** ACP inspects or updates the current session model
- **THEN** it SHALL use the runtime model helper methods rather than `ProviderRouter`

