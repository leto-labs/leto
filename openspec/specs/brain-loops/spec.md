# brain-loops Specification

## Purpose
Agent reasoning strategies. Defines the `AgentLoop` trait, `AgentConfig`, the typed `Event` stream, and the default `SimpleLoop` implementation (provider call, tool execution, re-inference loop).
## Requirements
### Requirement: AgentLoop Trait
The system SHALL define an `AgentLoop` trait with a single `run` method that accepts a Provider, a slice of Tools, a Vec of Messages, an AgentConfig, and a CancellationToken. It SHALL return a `Stream<Item = Event>`. Different implementations encode different agent strategies (simple tool loop, plan-then-act, multi-agent delegation, etc.).

The `AgentLoop` trait remains unchanged. However, callers SHOULD prefer invoking it through `Brain.turn()` rather than calling `run()` directly. `Brain.turn()` handles message history loading, user message construction, and post-turn persistence, whereas calling `AgentLoop.run()` directly requires the caller to manage all of this.

#### Scenario: Trait is object-safe and swappable
- **WHEN** a caller has a `&dyn AgentLoop`
- **THEN** it SHALL be able to call `run()` without knowing the concrete implementation

#### Scenario: Brain dispatches via AgentLoop
- **WHEN** `Brain.turn()` is called
- **THEN** it SHALL delegate to the configured `AgentLoop.run()` with the loaded message history, provider, tools, config, and cancellation token

### Requirement: SimpleLoop Implementation
The system SHALL provide a `SimpleLoop` as the default `AgentLoop` implementation. It calls the Provider, streams tokens, executes any tool calls, appends results, and loops until the model responds with no tool calls or a limit is reached.

#### Scenario: Simple text response
- **WHEN** SimpleLoop is run with a user message and no tools
- **THEN** it SHALL call the Provider, stream back Token events for each chunk, emit a MessageDone event with the full assistant message, and emit a TurnDone event

#### Scenario: Tool call and re-inference
- **WHEN** the Provider returns a tool call chunk
- **THEN** SimpleLoop SHALL emit ToolCallStart, execute the tool, emit ToolCallDone with the result, append the tool result as a message, and call the Provider again

#### Scenario: Max iterations reached
- **WHEN** the tool-call loop exceeds `config.max_iterations`
- **THEN** it SHALL emit an Error event and terminate the stream

#### Scenario: Cancellation
- **WHEN** the CancellationToken is cancelled during the loop
- **THEN** it SHALL emit an Error event with a Cancelled code and terminate

### Requirement: Event Stream
The agent loop SHALL emit a typed stream of `Event` variants:
- `Token { delta: String }` — a text fragment from the provider
- `ToolCallStart { id, name, arguments }` — tool dispatch begins
- `ToolCallDone { id, result }` — tool completed
- `MessageDone { message: Message }` — full assistant message committed
- `TurnDone { iterations, total_tokens }` — turn is complete
- `Error { code, message, recoverable }` — something went wrong

#### Scenario: Events arrive in order
- **WHEN** the provider streams "hello world" with no tool calls
- **THEN** the event stream SHALL yield: Token("hello"), Token(" "), Token("world"), MessageDone(...), TurnDone(...)

### Requirement: AgentConfig
The agent loop SHALL accept an `AgentConfig` with at minimum: `max_iterations` (default 20), `system_prompt` (optional String), and `inference: InferenceConfig`. The `InferenceConfig` carries `model: Option<String>`, `max_tokens: Option<u32>`, and `temperature: Option<f32>`. When `model` is `None`, the provider uses its own default.

#### Scenario: Default config
- **WHEN** AgentConfig::default() is used
- **THEN** max_iterations SHALL be 20, system_prompt SHALL be None, and inference.model SHALL be None

#### Scenario: Config propagates to provider
- **WHEN** `AgentConfig { inference: InferenceConfig { model: Some("gpt-4o"), .. }, .. }` is used
- **THEN** the agent loop SHALL pass this inference config to the provider's `chat()` method

### Requirement: RobustLoop Provides Hardened Turn Execution

The system SHALL provide a `RobustLoop` as an additive alternative to
`SimpleLoop`.

`RobustLoop` SHALL:

- retry transient provider failures before visible output is emitted
- detect repeated identical tool calls and emit doom-loop warnings
- compact older in-memory context when estimated usage exceeds a configured
  fraction of the active model's context window

#### Scenario: Transient provider failure is retried

- **WHEN** the provider returns a transient error before any visible output
- **THEN** `RobustLoop` SHALL emit a `Retry` event
- **AND** retry the provider call up to the configured maximum

#### Scenario: Repeated tool call triggers doom-loop mitigation

- **WHEN** the same tool name and arguments repeat past the configured threshold
- **THEN** `RobustLoop` SHALL emit a `DoomLoopWarning`
- **AND** either steer or terminate according to configuration

#### Scenario: Oversized context triggers compaction

- **WHEN** the estimated in-memory conversation exceeds the configured
  compaction threshold for the active model
- **THEN** `RobustLoop` SHALL summarize older messages into a replacement
  summary message
- **AND** emit a `Compaction` event

