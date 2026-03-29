# agent-loops Specification

## Purpose
Agent reasoning strategies. Defines the `AgentLoop` trait, `AgentConfig`, the typed `Event` stream, and the default `SimpleLoop` implementation (provider call, tool execution, re-inference loop).
## Requirements
### Requirement: AgentLoop Trait
The system SHALL define an `AgentLoop` trait with a single `run` method that accepts a Provider, a slice of Tools, a Vec of Messages, an AgentConfig, and a CancellationToken. It SHALL return a `Stream<Item = Event>`. Different implementations encode different agent strategies (simple tool loop, plan-then-act, multi-agent delegation, etc.).

The `AgentLoop` trait remains unchanged. However, callers SHOULD prefer
invoking it through the higher-level `agent-core` / `agent-runtime` turn flow
rather than calling `run()` directly. That turn path handles message history
loading, user message construction, and post-turn persistence, whereas calling
`AgentLoop.run()` directly requires the caller to manage all of this.

#### Scenario: Trait is object-safe and swappable
- **WHEN** a caller has a `&dyn AgentLoop`
- **THEN** it SHALL be able to call `run()` without knowing the concrete implementation

#### Scenario: Core dispatches via AgentLoop
- **WHEN** a higher-level core/runtime turn entrypoint is called
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
The Event enum SHALL be extended with the following new variants while
preserving all existing variants:

- `ToolCallDelta { id: String, name: String, arguments_delta: String }` — incremental tool call argument fragment from the provider stream
- `ToolCallPending { id: String, name: String, arguments: serde_json::Value }` — tool call is assembled and ready for execution; emitted before execution begins
- `ToolCallApproved { id: String }` — tool call was approved (by user or auto-approval policy)
- `ToolCallRejected { id: String, reason: String }` — tool call was rejected
- `Progress { phase: String, message: String, percent: Option<f32> }` — progress update for long-running operations
- `SessionStart { session_id: Ulid }` — a new session has been created
- `SessionResume { session_id: Ulid }` — an existing session has been resumed
- `Retry { attempt: u32, max: u32, error: String }` — transient provider retry information
- `Compaction { original_messages: usize, summary_tokens: usize }` — context compaction summary
- `DoomLoopWarning { tool_name: String, repetitions: u32 }` — repeated-tool warning

The Event enum SHALL be marked `#[non_exhaustive]` to allow future additions without breaking downstream consumers.

#### Scenario: Streaming tool call deltas
- **WHEN** the provider streams tool call argument fragments
- **THEN** the event stream SHALL yield `ToolCallDelta` for each fragment
- **AND** after the tool call is assembled, the loop SHALL still emit the normal tool execution lifecycle

#### Scenario: Auto-approved tool lifecycle
- **WHEN** no approval policy is configured
- **THEN** `ToolCallPending` SHALL still be emitted
- **AND** `ToolCallApproved` SHALL be emitted immediately (auto-approved)
- **AND** execution SHALL proceed without waiting for transport input

#### Scenario: Session lifecycle events
- **WHEN** the higher-level runtime flow creates a new session
- **THEN** it SHALL emit `SessionStart` before the first turn
- **WHEN** the higher-level runtime flow resumes an existing session
- **THEN** it SHALL emit `SessionResume` before the first turn

#### Scenario: Robust loop emits hardening events
- **WHEN** retry, compaction, or repeated-tool mitigation occurs
- **THEN** the event stream SHALL emit the corresponding `Retry`, `Compaction`,
  or `DoomLoopWarning` event

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

### Requirement: Terminus2Loop Provides Harbor-Style Text-Driven Terminal Control

The system SHALL provide a `Terminus2Loop` as an additive `AgentLoop`
implementation.

`Terminus2Loop` SHALL:

- call the provider with a Harbor-style plain-text prompt template rather than
  native tool definitions
- expect a structured JSON response containing `analysis`, `plan`, `commands`,
  and optional `task_complete`
- execute returned command batches against a persistent terminal session
- feed the resulting terminal state back as the next user prompt

#### Scenario: Text-driven command batch executes

- **WHEN** `Terminus2Loop` receives a valid structured assistant response with
  one or more commands
- **THEN** it SHALL emit a `MessageDone` event for the assistant response
- **AND** execute the requested command batch through the terminal-session
  primitive
- **AND** continue the loop using the resulting terminal observation

### Requirement: Terminus2Loop Reprompts On Parse Failure

`Terminus2Loop` SHALL preserve Harbor Terminus-2 parse-recovery semantics.

#### Scenario: Invalid structured response triggers repair prompt

- **WHEN** the provider response is missing required fields or contains invalid
  command JSON
- **THEN** `Terminus2Loop` SHALL record the raw assistant response
- **AND** reprompt the provider with a parse-repair instruction
- **AND** SHALL NOT execute any commands from the invalid response

### Requirement: Terminus2Loop Requires Completion Confirmation

`Terminus2Loop` SHALL require the model to confirm task completion twice before
 ending the turn.

#### Scenario: First completion request asks for confirmation

- **WHEN** the provider marks `task_complete` on a turn that was not already
  pending completion
- **THEN** the loop SHALL return a completion-confirmation prompt containing the
  current terminal state
- **AND** continue running rather than ending the turn immediately

#### Scenario: Second completion request ends the turn

- **WHEN** the provider marks `task_complete` while completion confirmation is
  already pending
- **THEN** the loop SHALL emit `TurnDone` and terminate successfully

### Requirement: Terminus2Loop Summarizes Under Context Pressure

`Terminus2Loop` SHALL preserve Harbor Terminus-2 summarization behavior as an
 additive loop strategy.

#### Scenario: Proactive summarization occurs near context limit

- **WHEN** estimated free context drops below the configured proactive
  summarization threshold
- **THEN** `Terminus2Loop` SHALL perform the summarization handoff flow before
  the next main provider turn

#### Scenario: Context-limit failure triggers fallback summarization

- **WHEN** the provider reports a classified context-length failure
- **THEN** `Terminus2Loop` SHALL attempt summarization-based recovery before
  giving up on the turn

### Requirement: Terminus2Loop Uses Bounded Retry Policy

`Terminus2Loop` SHALL retry ordinary inference failures, but SHALL NOT retry
 cancellation or classified context-length failures directly.

#### Scenario: Ordinary inference failure retries

- **WHEN** the provider returns a retryable inference failure during a normal
  Terminus-2 provider call
- **THEN** the loop SHALL retry up to the configured retry limit

#### Scenario: Cancellation stops without retry

- **WHEN** cancellation is observed during a Terminus-2 provider call
- **THEN** the loop SHALL terminate without retrying the cancelled request

### Requirement: TerminusKiraLoop Provides Native Tool-Calling Terminal Control

The system SHALL provide a `TerminusKiraLoop` as an additive `AgentLoop`
implementation.

`TerminusKiraLoop` SHALL:

- call the provider with native semantic tool definitions rather than a
  text-only JSON protocol
- expose KIRA-style semantic tools named `execute_commands`, `task_complete`,
  and `image_read`
- execute command batches against a persistent terminal session
- support marker-based polling so command batches may complete before the full
  requested wait duration

#### Scenario: Execute commands tool call drives terminal batch

- **WHEN** the provider returns an `execute_commands` tool call
- **THEN** `TerminusKiraLoop` SHALL emit the assistant message and tool-call
  events
- **AND** execute the requested command batch through the terminal-session
  primitive
- **AND** continue the loop using the resulting observation

### Requirement: TerminusKiraLoop Supports Multimodal Image Read

`TerminusKiraLoop` SHALL support a KIRA-style `image_read` action by issuing a
multimodal provider subcall with text plus image content.

#### Scenario: Image read analyzes png file

- **WHEN** the provider returns an `image_read` request for a PNG file with an
  analysis instruction
- **THEN** the loop SHALL read and encode the image
- **AND** perform a multimodal provider call
- **AND** feed the resulting analysis back into the main loop as the next
  observation

### Requirement: TerminusKiraLoop Requires Double Completion Confirmation

`TerminusKiraLoop` SHALL require a second explicit completion confirmation
before ending the turn.

#### Scenario: First completion request returns checklist

- **WHEN** the provider requests `task_complete` while completion is not yet
  pending
- **THEN** the loop SHALL return a completion-confirmation checklist and
  continue running

#### Scenario: Second completion request ends turn

- **WHEN** the provider requests `task_complete` while completion is already
  pending
- **THEN** the loop SHALL emit `TurnDone` and terminate successfully
