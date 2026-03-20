# brain-acp Specification Delta

## ADDED Requirements

### Requirement: Real Backend Exposes ACP Config Options

The real `brain-acp` backend SHALL expose ACP `configOptions` for session
settings.

The first real surface SHALL include:

- `model`
- `thought_level`

The backend SHALL include those config options on both `session/new` and
`session/load`.

#### Scenario: New session includes config options
- **WHEN** ACP creates a new real backend session
- **THEN** the response SHALL include ACP `configOptions`

#### Scenario: Loaded session includes config options
- **WHEN** ACP loads a real backend session
- **THEN** the response SHALL include ACP `configOptions`

### Requirement: Real Backend Supports Session Config Mutation

The real `brain-acp` backend SHALL implement `session/set_config_option`.

The backend SHALL:

- accept `model` updates
- accept `thought_level` updates when supported by the effective model
- return the full current config-option snapshot after each update

#### Scenario: Model update returns full config options
- **WHEN** ACP sets the real backend session `model`
- **THEN** the backend SHALL persist the session inference update
- **AND** return the full current `configOptions`

#### Scenario: Thought-level update returns full config options
- **WHEN** ACP sets the real backend session `thought_level`
- **THEN** the backend SHALL persist the session inference update
- **AND** return the full current `configOptions`

### Requirement: Real Backend Groups Model Options By Provider

The real backend `model` config option SHALL be exposed as a provider-grouped
select.

#### Scenario: Model config option preserves provider grouping
- **WHEN** ACP inspects real backend config options
- **THEN** the `model` option SHALL group choices by provider
- **AND** preserve the current runtime model ordering within each provider group
