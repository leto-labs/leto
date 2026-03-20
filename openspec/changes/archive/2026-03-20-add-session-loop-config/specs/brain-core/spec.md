## MODIFIED Requirements
### Requirement: Native Runtime Turn Resolution

`BrainRuntimeNative` SHALL resolve its provider, tools, and loop directly from
its registries rather than routing execution through `ProviderRouter`.

Provider resolution SHALL:

- honor explicit `InferenceConfig.provider`
- otherwise resolve a unique provider by `InferenceConfig.model`
- otherwise fall back to the configured default provider

Loop resolution SHALL:

- honor the effective loop name from session/project config
- otherwise fall back to the configured default loop

Tool collection SHALL use all registered tools.

#### Scenario: Session loop helper persists an override
- **WHEN** `set_session_loop()` updates a session to a registered loop
- **THEN** the persisted session SHALL store that loop override

#### Scenario: Invalid session loop is rejected
- **WHEN** `set_session_loop()` is called with an unregistered loop name
- **THEN** the runtime SHALL reject the update
