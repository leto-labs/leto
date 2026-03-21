# brain-core Specification

## MODIFIED Requirements

### Requirement: Brain Turn Method

The `Brain` SHALL provide a `turn` method that executes a single conversational
turn:
1. Load the session for the given session ID
2. Load the owning project and derive the base `AgentConfig`
3. Merge any session inference overrides into the project's inference defaults
4. Load existing messages from the store
5. Append a new user message
6. Run the agent loop with the full message history and the effective config
7. Collect all assistant and tool messages emitted during the turn
8. Persist the new messages (user + assistant + tool results) to the store
9. Return the event stream to the caller

The method signature SHALL be:
`turn(&self, session_id: Ulid, input: &str, cancel: CancellationToken) -> EventStream`

#### Scenario: Turn uses effective session inference

- **WHEN** a session has inference overrides and `turn()` is called
- **THEN** the agent loop SHALL receive the project defaults merged with those session overrides

### Requirement: ProviderRouter

The system SHALL provide a `ProviderRouter` struct in `brain-core` that
implements the `Provider` trait.

It SHALL:

- register providers by provider name
- resolve `InferenceConfig.provider` explicitly when present
- resolve `InferenceConfig.model` only when that model maps to exactly one
  registered provider
- fall back to the configured default provider only when neither provider nor
  model is set

#### Scenario: Explicit provider wins

- **WHEN** `chat()` is called with `InferenceConfig { provider: Some("openai"), .. }`
- **THEN** the router SHALL dispatch to the registered `openai` provider

#### Scenario: Model-only routing requires unique ownership

- **WHEN** `chat()` is called with `InferenceConfig { provider: None, model: Some("gpt-4o") }`
- **AND** exactly one registered provider advertises `"gpt-4o"`
- **THEN** the router SHALL dispatch to that provider

#### Scenario: Ambiguous model-only routing errors

- **WHEN** a model ID belongs to more than one registered provider
- **THEN** model-only routing for that ID SHALL fail instead of silently picking one

#### Scenario: Available models preserve provider-declared order

- **WHEN** `ProviderRouter` exposes its available model list for transports or clients
- **THEN** it SHALL preserve the order declared by each registered provider
- **AND** it SHALL exclude model IDs that are ambiguous across providers

## ADDED Requirements

### Requirement: Effective Session Inference Resolution

`Brain` SHALL provide helpers for resolving and updating session inference
state.

#### Scenario: Effective inference merges project and session state

- **WHEN** a project default sets provider/model and the session override sets only model
- **THEN** the effective inference SHALL keep the project provider and use the session model

#### Scenario: Empty session override is cleared

- **WHEN** a caller updates session inference with an empty override config
- **THEN** `Brain` SHALL persist no session override layer for that session
