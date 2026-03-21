# brain-loops Specification Delta

## ADDED Requirements

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
