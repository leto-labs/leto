# brain-core Specification

## Purpose
Core engine and native runtime orchestration for `brain`. `Brain` remains the
reusable reasoning engine API, while `BrainRuntimeNative` provides the
app-facing runtime boundary over stores, providers, tools, loops, and runtime
events.
## Requirements
### Requirement: Brain Struct
The system SHALL provide a `Brain` struct in `brain-core` that orchestrates all engine components. It SHALL hold:
- `provider: Arc<dyn Provider>` — the LLM inference backend
- `store: Arc<dyn Store>` — session and message persistence
- `agent_loop: Arc<dyn AgentLoop>` — the agent strategy
- `tools: Vec<Arc<dyn Tool>>` — available tools

Brain does NOT hold a `Project` or `AgentConfig`. It is a reusable engine; project context is supplied by callers at the method level.

#### Scenario: Construction
- **WHEN** `Brain::new(provider, store, agent_loop, tools)` is called
- **THEN** it SHALL return a Brain instance holding all provided components

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

### Requirement: Brain Run Loop
The `Brain` SHALL provide a `run` method that drives an interactive session using a Transport and a Project:
1. Create a new session via the store, scoped to the project's ID
2. Derive `AgentConfig` from the project's config
3. Loop: call `transport.recv()` for the next input
4. On `InputEvent::Message`, call `turn()` with the session, config, and forward each event to `transport.send()`
5. On `None` (EOF), exit the loop cleanly

The method signature SHALL be:
`run(&self, project: &Project, transport: &dyn Transport) -> Result<(), BrainError>`

#### Scenario: Interactive session lifecycle
- **WHEN** `Brain.run(project, transport)` is called
- **THEN** it SHALL create a session scoped to the project, read input from the transport, dispatch turns with the project's config, and send events back through the transport until EOF

#### Scenario: Clean shutdown on EOF
- **WHEN** the transport returns `None` from `recv()`
- **THEN** `run()` SHALL return `Ok(())`

### Requirement: Brain Session Delegation
The `Brain` SHALL expose session management methods that delegate to the underlying store. These methods take a `project_id` parameter for scoping:
- `create_session(project_id: ProjectId) -> Result<Session, BrainError>`
- `list_sessions(project_id: ProjectId) -> Result<Vec<Session>, BrainError>`

These allow callers to manage sessions without accessing the store directly.

#### Scenario: Create session through Brain
- **WHEN** `brain.create_session(project_id)` is called
- **THEN** it SHALL delegate to `store.session_create(project_id)` and return the result

#### Scenario: List sessions through Brain
- **WHEN** `brain.list_sessions(project_id)` is called
- **THEN** it SHALL delegate to `store.session_list(project_id)` and return the result

### Requirement: Re-export Facade Preserved
`brain-core` SHALL continue to re-export the runtime-facing crates used by the
local CLI path. The old `cli-echo` and `cli-local` binaries have been removed
from the workspace because their behavior is subsumed by `brain-cli`.

#### Scenario: Workspace members updated
- **WHEN** the focused runtime workspace is built
- **THEN** `brain-cli` SHALL be the user-facing local binary crate
- **AND** `examples/cli-echo` and `examples/cli-local` SHALL NOT be present

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

#### Scenario: Session loop helper persists an override
- **WHEN** `set_session_loop()` updates a session to a registered loop
- **THEN** the persisted session SHALL store that loop override

#### Scenario: Invalid session loop is rejected
- **WHEN** `set_session_loop()` is called with an unregistered loop name
- **THEN** the runtime SHALL reject the update

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

### Requirement: Session Inference Supports Thought Level Overrides

The core runtime SHALL treat thought level as part of session-scoped inference.

The core SHALL:

- merge project default thought level with session overrides
- validate thought-level updates against the current effective model's supported reasoning levels
- omit the effective thought level when the current model has no reasoning capability

#### Scenario: Effective thought level merges from project and session
- **WHEN** a project has a default thought level and a session sets a different thought level
- **THEN** the effective session inference SHALL use the session value

#### Scenario: Invalid thought level is rejected
- **WHEN** a caller attempts to set a thought level unsupported by the current model
- **THEN** the runtime SHALL reject the update

#### Scenario: Non-reasoning model clears effective thought level
- **WHEN** the effective session model does not advertise reasoning support
- **THEN** the effective session inference SHALL have no thought-level value

### Requirement: Effective Session Inference Resolution

`Brain` SHALL provide helpers for resolving and updating session inference
state.

#### Scenario: Effective inference merges project and session state

- **WHEN** a project default sets provider/model and the session override sets only model
- **THEN** the effective inference SHALL keep the project provider and use the session model

#### Scenario: Empty session override is cleared

- **WHEN** a caller updates session inference with an empty override config
- **THEN** `Brain` SHALL persist no session override layer for that session
