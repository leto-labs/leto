# brain-loops Delta Spec

## MODIFIED Requirements

### Requirement: SimpleLoop Implementation
The system SHALL provide a `SimpleLoop` as the default `AgentLoop`
implementation. It calls the Provider, streams tokens, executes any tool calls,
appends results, and loops until the model responds with no tool calls or a
limit is reached.

Successful turns SHALL terminate with `TurnDone`. User-initiated cancellation
SHALL terminate with `Interrupted`. Runtime failures SHALL terminate with
`Error`.

#### Scenario: Simple text response
- **WHEN** `SimpleLoop` is run with a user message and no tools
- **THEN** it SHALL call the Provider, stream back `Token` events for each chunk, emit `MessageDone` with the full assistant message, and emit `TurnDone`

#### Scenario: Tool call and re-inference
- **WHEN** the Provider returns a tool call chunk
- **THEN** `SimpleLoop` SHALL emit `ToolCallStart`, execute the tool, emit `ToolCallDone` with the result, append the tool result as a message, and call the Provider again

#### Scenario: Max iterations reached
- **WHEN** the tool-call loop exceeds `config.max_iterations`
- **THEN** it SHALL emit an `Error` event and terminate the stream

#### Scenario: Cancellation
- **WHEN** the `CancellationToken` is cancelled during the loop
- **THEN** it SHALL emit `Interrupted` and terminate
- **AND** it SHALL NOT emit a generic cancellation `Error`

### Requirement: Event Stream
The agent loop SHALL emit a typed stream of `Event` variants:

- `Token { delta: String }` — a text fragment from the provider
- `ToolCallStart { id, name, arguments }` — tool dispatch begins
- `ToolCallDone { id, result, is_error }` — tool completed
- `MessageDone { message: Message }` — full assistant message committed
- `TurnDone { iterations, total_tokens }` — turn completed successfully
- `Interrupted` — turn was cancelled by the caller
- `Error { code, message, recoverable }` — runtime failure

For each turn, the stream SHALL terminate with exactly one terminal event:
`TurnDone`, `Interrupted`, or `Error`.

#### Scenario: Events arrive in order
- **WHEN** the provider streams `"hello world"` with no tool calls
- **THEN** the event stream SHALL yield `Token("hello")`, `Token(" ")`, `Token("world")`, `MessageDone(...)`, `TurnDone(...)`

#### Scenario: Interrupted after partial output
- **WHEN** the provider has already emitted some tokens and the caller cancels the turn
- **THEN** the already-emitted `Token` events SHALL remain part of the stream history
- **AND** the stream SHALL terminate with `Interrupted`

#### Scenario: Interrupted during tool work
- **WHEN** cancellation occurs after tool-related events have already been emitted
- **THEN** those previously emitted tool events SHALL remain valid
- **AND** the turn SHALL still terminate with `Interrupted`
