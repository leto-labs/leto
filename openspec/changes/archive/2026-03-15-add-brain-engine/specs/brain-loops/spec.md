# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

### Requirement: AgentLoop Trait
The system SHALL define an `AgentLoop` trait with a single `run` method that accepts a Provider, a slice of Tools, a Vec of Messages, an AgentConfig, and a CancellationToken. It SHALL return a `Stream<Item = Event>`. Different implementations encode different agent strategies (simple tool loop, plan-then-act, multi-agent delegation, etc.).

#### Scenario: Trait is object-safe and swappable
- **WHEN** a caller has a `&dyn AgentLoop`
- **THEN** it SHALL be able to call `run()` without knowing the concrete implementation

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
The agent loop SHALL accept an `AgentConfig` with at minimum: `max_iterations` (default 20) and `system_prompt` (optional String).

#### Scenario: Default config
- **WHEN** AgentConfig::default() is used
- **THEN** max_iterations SHALL be 20 and system_prompt SHALL be None
