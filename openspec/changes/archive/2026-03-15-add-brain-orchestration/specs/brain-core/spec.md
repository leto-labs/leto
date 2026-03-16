# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: Brain Struct
The system SHALL provide a `Brain` struct in `brain-core` that orchestrates all engine components. It SHALL hold:
- `provider: Arc<dyn Provider>` — the LLM inference backend
- `store: Arc<dyn Store>` — session and message persistence
- `agent_loop: Arc<dyn AgentLoop>` — the agent strategy
- `tools: Vec<Arc<dyn Tool>>` — available tools
- `config: AgentConfig` — agent configuration (max iterations, system prompt, inference settings)

#### Scenario: Construction
- **WHEN** `Brain::new(provider, store, agent_loop, tools, config)` is called
- **THEN** it SHALL return a Brain instance holding all provided components

### Requirement: Brain Turn Method
The `Brain` SHALL provide a `turn` method that executes a single conversational turn:
1. Load existing messages from the store for the given session
2. Append a new user message
3. Run the agent loop with the full message history
4. Collect all assistant and tool messages emitted during the turn
5. Persist the new messages (user + assistant + tool results) to the store
6. Return the event stream to the caller

The method signature SHALL be:
`turn(&self, session_id: Ulid, input: &str, cancel: CancellationToken) -> EventStream`

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
The `Brain` SHALL provide a `run` method that drives an interactive session using a Transport:
1. Create a new session via the store
2. Loop: call `transport.recv()` for the next input
3. On `InputEvent::Message`, call `turn()` with the session and forward each event to `transport.send()`
4. On `None` (EOF), exit the loop cleanly

The method signature SHALL be:
`run(&self, transport: &dyn Transport) -> Result<(), BrainError>`

#### Scenario: Interactive session lifecycle
- **WHEN** `Brain.run(transport)` is called
- **THEN** it SHALL create a session, read input from the transport, dispatch turns, and send events back through the transport until EOF

#### Scenario: Clean shutdown on EOF
- **WHEN** the transport returns `None` from `recv()`
- **THEN** `run()` SHALL return `Ok(())`

### Requirement: Brain Session Delegation
The `Brain` SHALL expose session management methods that delegate to the underlying store:
- `create_session() -> Result<Session, BrainError>`
- `list_sessions() -> Result<Vec<Session>, BrainError>`

These allow callers to manage sessions without accessing the store directly.

#### Scenario: Create session through Brain
- **WHEN** `brain.create_session()` is called
- **THEN** it SHALL delegate to `store.create_session()` and return the result

#### Scenario: List sessions through Brain
- **WHEN** `brain.list_sessions()` is called
- **THEN** it SHALL delegate to `store.list_sessions()` and return the result

### Requirement: Re-export Facade Preserved
`brain-core` SHALL continue to re-export all public items from `brain-types`, `brain-providers`, `brain-stores`, `brain-loops`, and `brain-transports`. Downstream consumers using `use brain_core::*` SHALL not break.

#### Scenario: Existing imports still work
- **WHEN** a consumer uses `use brain_core::*`
- **THEN** all types, traits, and implementations from sub-crates SHALL be accessible
- **AND** the `Brain` struct SHALL also be accessible
