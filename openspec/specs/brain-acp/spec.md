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

### Requirement: Real Backend Exposes ACP Config Options

The real `brain-acp` backend SHALL expose ACP `configOptions` for session
settings.

The real surface SHALL include:

- `model`
- `thought_level`
- `loop` when more than one loop is registered

The backend SHALL include those config options on both `session/new` and
`session/load`.

#### Scenario: Real backend omits loop config when no alternative exists
- **WHEN** ACP creates or loads a real backend session with only one registered loop
- **THEN** the response SHALL omit the `loop` config option

#### Scenario: Real backend exposes loop config when multiple loops are registered
- **WHEN** ACP creates or loads a real backend session with multiple registered loops
- **THEN** the response SHALL include a `loop` config option

### Requirement: Real Backend Supports Session Config Mutation

The real `brain-acp` backend SHALL implement `session/set_config_option`.

The backend SHALL:

- accept `model` updates
- accept `thought_level` updates when supported by the effective model
- accept `loop` updates when the named loop is registered
- return the full current config-option snapshot after each update

#### Scenario: Loop update returns full config options
- **WHEN** ACP sets the real backend session `loop`
- **THEN** the backend SHALL persist the session loop override
- **AND** return the full current `configOptions`

### Requirement: Real Backend Groups Model Options By Provider

The real backend `model` config option SHALL be exposed as a provider-grouped
select.

#### Scenario: Model config option preserves provider grouping
- **WHEN** ACP inspects real backend config options
- **THEN** the `model` option SHALL group choices by provider
- **AND** preserve the current runtime model ordering within each provider group

