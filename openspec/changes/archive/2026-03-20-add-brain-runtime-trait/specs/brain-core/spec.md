# brain-core Delta Spec

## ADDED Requirements

### Requirement: BrainRuntimeNative
The system SHALL provide a `BrainRuntimeNative` type in `brain-core` as the
first concrete implementation of `BrainRuntime`.

`BrainRuntimeNative` SHALL own:

- a single `Store`
- a provider registry
- a tool registry
- a loop registry
- one configured default provider name
- one configured default loop name
- active-turn cancellation state keyed by session ID

#### Scenario: Native runtime is available from brain-core
- **WHEN** a caller imports `brain_core::BrainRuntimeNative`
- **THEN** the type SHALL resolve from the crate root

#### Scenario: Native runtime implements BrainRuntime
- **WHEN** a caller constructs a `BrainRuntimeNative`
- **THEN** it SHALL expose the `BrainRuntime` interface over its store and registries

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

#### Scenario: Turn uses default provider and loop
- **WHEN** no explicit provider, model, or loop override is configured
- **THEN** the runtime SHALL execute the turn with its configured default provider and default loop

#### Scenario: Turn resolves provider by model without router
- **WHEN** a model is configured and exactly one registered provider advertises that model
- **THEN** the runtime SHALL use that provider

#### Scenario: Ambiguous model selection fails
- **WHEN** multiple registered providers advertise the requested model
- **THEN** the runtime SHALL emit an error rather than arbitrarily selecting one

#### Scenario: Session override selects a different loop
- **WHEN** a session has a loop override configured
- **THEN** the runtime SHALL use the named registered loop for that turn

### Requirement: Native Runtime Cancellation
`BrainRuntimeNative` SHALL own active-turn cancellation for its sessions.

#### Scenario: Concurrent second turn is rejected
- **WHEN** a session already has an active turn
- **THEN** starting another turn for the same session SHALL produce a `TurnActive` error

#### Scenario: Cancel turn cancels active work
- **WHEN** `cancel_turn(session_id)` is called for an active session
- **THEN** the runtime SHALL cancel the stored token for that turn

### Requirement: Native Runtime Bus
`BrainRuntimeNative` SHALL provide a live shared runtime bus for protocol-neutral
observation of runtime activity.

The runtime bus SHALL:

- expose a subscription API on the runtime
- publish duplexed turn events with session context
- compose lifecycle events from the underlying store event stream
- remain live-only rather than replaying persisted history

#### Scenario: Subscribers receive duplexed turn events
- **WHEN** a caller subscribes to the runtime bus and starts a turn
- **THEN** the subscriber SHALL receive wrapped turn events with the originating session ID

#### Scenario: Project creation publishes a lifecycle event
- **WHEN** `resolve_or_create_project()` creates a new project
- **THEN** the runtime bus SHALL publish a project-created event

#### Scenario: Session model update publishes a lifecycle event
- **WHEN** `set_session_model()` updates persisted session inference
- **THEN** the runtime bus SHALL publish a session-updated event with the updated session

### Requirement: Native Runtime Project Bootstrap
`BrainRuntimeNative` SHALL own project bootstrap when resolving a project root.

When creating a missing project from a filesystem root, the runtime SHALL:

- normalize the root path
- load filesystem config for that root
- use the root `.agents/AGENTS.md` as `system_prompt` only when config does not already set one

#### Scenario: Project bootstrap loads config and root agents prompt
- **WHEN** the runtime resolves a new project root with a local `.agents/config.toml` and `.agents/AGENTS.md`
- **THEN** the created project SHALL include the resolved config values
- **AND** use the root agents prompt when no explicit system prompt is configured

#### Scenario: Explicit config prompt wins over AGENTS.md
- **WHEN** the runtime resolves a new project root whose config already declares `system_prompt`
- **THEN** the created project SHALL preserve that configured prompt
