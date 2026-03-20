## MODIFIED Requirements
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
