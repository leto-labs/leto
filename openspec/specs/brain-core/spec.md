# brain-core Specification

## Purpose
Central orchestration engine for the AI agent. The `Brain` struct coordinates Provider, Store, AgentLoop, and Tool into a reusable reasoning engine. Project context (config, session scoping) is passed at call sites rather than owned by Brain. `ProviderRouter` enables multi-provider dispatch by model name.
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
The `Brain` SHALL provide a `turn` method that executes a single conversational turn:
1. Load existing messages from the store for the given session
2. Append a new user message
3. Run the agent loop with the full message history and the provided `AgentConfig`
4. Collect all assistant and tool messages emitted during the turn
5. Persist the new messages (user + assistant + tool results) to the store
6. Return the event stream to the caller

The method signature SHALL be:
`turn(&self, session_id: Ulid, input: &str, config: AgentConfig, cancel: CancellationToken) -> EventStream`

#### Scenario: First turn in new session
- **WHEN** `turn()` is called on a session with no prior messages
- **THEN** the agent loop SHALL receive only the user message (plus system prompt if configured)
- **AND** the user message and all assistant/tool messages SHALL be persisted to the store

#### Scenario: Subsequent turn with history
- **WHEN** `turn()` is called on a session with prior messages
- **THEN** the agent loop SHALL receive the full history plus the new user message
- **AND** only the new messages SHALL be appended to the store

#### Scenario: Turn with tool calls
- **WHEN** the agent loop invokes tools during a turn
- **THEN** all tool result messages SHALL also be persisted to the store

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
`brain-core` SHALL continue to re-export all public items from `brain-types`, `brain-providers`, `brain-stores`, `brain-loops`, and `brain-transports`. Downstream consumers using `use brain_core::*` SHALL not break.

#### Scenario: Existing imports still work
- **WHEN** a consumer uses `use brain_core::*`
- **THEN** all types, traits, and implementations from sub-crates SHALL be accessible
- **AND** the `Brain` struct SHALL also be accessible

### Requirement: ProviderRouter
The system SHALL provide a `ProviderRouter` struct in `brain-core` that implements the `Provider` trait. It SHALL hold a map of model names to `Arc<dyn Provider>` instances and a default provider. When `chat()` is called, it SHALL read `InferenceConfig.model` to look up the target provider, falling back to the default if the model is `None` or not found in the map.

#### Scenario: Route by model name
- **WHEN** `chat()` is called with `InferenceConfig { model: Some("gpt-4o") }`
- **AND** a provider is registered under the name `"gpt-4o"`
- **THEN** the call SHALL be dispatched to that provider

#### Scenario: Fallback to default
- **WHEN** `chat()` is called with `InferenceConfig { model: None }`
- **THEN** the call SHALL be dispatched to the default provider

#### Scenario: Unknown model falls back
- **WHEN** `chat()` is called with a model name not registered in the router
- **THEN** the call SHALL be dispatched to the default provider

#### Scenario: Construction
- **WHEN** `ProviderRouter::new(default)` is called and providers are added via `add(name, provider)`
- **THEN** the router SHALL be usable as an `Arc<dyn Provider>` passed to `Brain::new()`

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

