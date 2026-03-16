# brain-loops Delta Spec

## MODIFIED Requirements

### Requirement: AgentConfig
AgentConfig SHALL be extended with the following fields:
- `doom_loop_threshold: Option<u32>` — number of identical tool call repetitions before triggering mitigation (default: None = disabled in SimpleLoop, 3 in RobustLoop)
- `compaction_threshold: Option<f32>` — fraction of estimated context window usage that triggers compaction (e.g., 0.8 = 80%)
- `compaction_model: Option<String>` — model to use for summarization during compaction (defaults to the current inference model)
- `max_retries: Option<u32>` — max retry attempts for transient provider errors (default: None = disabled in SimpleLoop, 3 in RobustLoop)
- `retry_base_delay_ms: Option<u64>` — base delay for exponential backoff (default: 1000)

Existing fields (`max_iterations`, `system_prompt`, `inference`) SHALL remain unchanged.

#### Scenario: Default config unchanged
- **WHEN** `AgentConfig::default()` is used
- **THEN** all new fields SHALL be `None`, preserving current SimpleLoop behavior

#### Scenario: RobustLoop config
- **WHEN** a config is created for RobustLoop
- **THEN** `doom_loop_threshold`, `max_retries`, and `compaction_threshold` SHALL have non-None defaults

## ADDED Requirements

### Requirement: Doom-Loop Detection
The agent loop SHALL optionally detect when the model is stuck in a repetitive tool-call pattern. Detection works by maintaining a sliding window of recent tool calls (name + arguments). When the same call appears more than `doom_loop_threshold` consecutive times, the loop SHALL take corrective action.

#### Scenario: Repeated tool call detected
- **WHEN** the model calls `file_read({"path": "foo.txt"})` three consecutive times and `doom_loop_threshold` is 3
- **THEN** the loop SHALL emit a warning event
- **AND** inject a system message telling the model to try a different approach

#### Scenario: Repeated pattern terminates
- **WHEN** doom-loop mitigation via system message fails and repetition continues for another `doom_loop_threshold` calls
- **THEN** the loop SHALL emit an Error event and terminate the turn

#### Scenario: Detection disabled
- **WHEN** `doom_loop_threshold` is `None`
- **THEN** no doom-loop detection SHALL occur (current SimpleLoop behavior)

### Requirement: Context Compaction
The agent loop SHALL optionally compact message history when it approaches the model's context window limit. Compaction summarizes older messages into a condensed form using a provider call.

#### Scenario: Compaction triggered
- **WHEN** the estimated token count of the message history exceeds `compaction_threshold` fraction of the context window
- **THEN** the loop SHALL select the oldest messages (excluding system prompt and the latest turn)
- **AND** summarize them using the configured `compaction_model` (or current model)
- **AND** replace the selected messages with a single summary message
- **AND** emit a `Compaction` event

#### Scenario: Compaction preserves recent context
- **WHEN** compaction is triggered
- **THEN** the system prompt, the latest user message, and all messages from the current turn SHALL NOT be compacted

#### Scenario: Compaction disabled
- **WHEN** `compaction_threshold` is `None`
- **THEN** no compaction SHALL occur

### Requirement: Retry with Backoff
The agent loop SHALL optionally retry failed provider calls for transient errors. Retries SHALL use exponential backoff with jitter.

#### Scenario: Transient error retried
- **WHEN** a provider call fails with a transient error (429, 500, 502, 503, network timeout)
- **AND** `max_retries` is set and the retry count has not been exceeded
- **THEN** the loop SHALL wait with exponential backoff and retry the call

#### Scenario: Max retries exceeded
- **WHEN** a provider call fails and the retry count exceeds `max_retries`
- **THEN** the loop SHALL emit an Error event and terminate the turn

#### Scenario: Non-transient error not retried
- **WHEN** a provider call fails with a non-transient error (400, 401, 403)
- **THEN** the loop SHALL NOT retry and SHALL emit an Error event immediately

#### Scenario: Retry disabled
- **WHEN** `max_retries` is `None`
- **THEN** no retry SHALL occur (current SimpleLoop behavior)

### Requirement: RobustLoop
The system SHALL provide a `RobustLoop` implementation of `AgentLoop` that includes doom-loop detection, context compaction, and retry with backoff. It SHALL use the same fundamental tool-loop strategy as `SimpleLoop` but with these hardening features enabled by default.

#### Scenario: RobustLoop with defaults
- **WHEN** `RobustLoop` is used with `AgentConfig::default()`
- **THEN** doom-loop detection (threshold=3), retry (max_retries=3), and compaction (threshold=0.8) SHALL all be active

#### Scenario: RobustLoop respects config overrides
- **WHEN** `RobustLoop` is used with custom config values
- **THEN** the custom values SHALL override the defaults

### Requirement: New Event Variants
The Event enum SHALL be extended with:
- `Retry { attempt: u32, max: u32, error: String }` — a retry is being attempted
- `Compaction { original_messages: usize, summary_tokens: usize }` — context was compacted
- `DoomLoopWarning { tool_name: String, repetitions: u32 }` — repetitive pattern detected

#### Scenario: Retry event emitted
- **WHEN** a retry is attempted
- **THEN** the event stream SHALL yield a `Retry` event before the retry delay

#### Scenario: Compaction event emitted
- **WHEN** context compaction occurs
- **THEN** the event stream SHALL yield a `Compaction` event with details

