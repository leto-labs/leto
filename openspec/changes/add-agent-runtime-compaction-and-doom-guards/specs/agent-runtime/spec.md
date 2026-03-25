## ADDED Requirements

### Requirement: Agent Runtime Computes Advisory Context Pressure

The runtime SHALL expose advisory transcript-pressure state so loops can decide
when to compact context without reimplementing provider-limit estimation.

#### Scenario: Loop sees transcript pressure before provider execution

- **WHEN** the runtime knows the active model context limit and transcript size
- **THEN** it SHALL expose transcript-pressure advice through loop-visible
  session state
- **AND** indicate whether compaction is recommended for the current turn

### Requirement: Agent Runtime Executes Transcript Compaction Safely

The runtime SHALL implement transcript compaction as a runtime-owned action
behind the public loop decision surface.

#### Scenario: Loop requests compaction

- **WHEN** a loop returns `CompactContext`
- **THEN** the runtime SHALL summarize the compactable transcript middle through
  a provider subcall
- **AND** preserve leading instruction messages and a recent trailing window
- **AND** replace the compacted middle with one runtime-authored summary
  message
- **AND** emit a compaction event

#### Scenario: Compaction failure leaves transcript intact

- **WHEN** summary generation fails
- **THEN** the runtime SHALL leave the transcript unchanged
- **AND** surface a runtime error without corrupting turn state

### Requirement: Agent Runtime Detects Repeated Tool-Call Doom Loops

The runtime SHALL track repeated tool-call signatures and expose advisory
doom-loop state to loops and observers.

#### Scenario: Runtime emits advisory doom-loop warning

- **WHEN** repeated identical tool invocations cross the configured advisory
  threshold
- **THEN** the runtime SHALL expose doom-loop state in the session snapshot
- **AND** emit a doom-loop warning event

#### Scenario: Runtime enforces optional hard-stop threshold

- **WHEN** repeated identical tool invocations cross a configured hard-stop
  threshold
- **THEN** the runtime SHALL abort the turn with an error rather than continue
  indefinitely

### Requirement: Agent Runtime Lets Loops Queue Local Steering

The runtime SHALL let loop strategies queue steering for the current runtime
without going through an external command path.

#### Scenario: Loop queues steering after advisory warning

- **WHEN** a loop decides to steer the current runtime
- **THEN** it SHALL be able to request local steering through the public loop
  decision surface
- **AND** the runtime SHALL inject that steering at the requested safe boundary
