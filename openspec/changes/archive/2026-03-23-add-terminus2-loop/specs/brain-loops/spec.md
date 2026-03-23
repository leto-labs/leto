## ADDED Requirements

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
