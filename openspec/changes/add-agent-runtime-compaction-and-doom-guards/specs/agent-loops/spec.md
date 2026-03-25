## MODIFIED Requirements

### Requirement: SimpleLoop Uses Reusable Runtime Helpers

`SimpleLoop` SHALL use the public reusable runtime surface rather than private
provider orchestration or transcript mutation shortcuts.

#### Scenario: Simple loop stays within runtime boundary

- **WHEN** `SimpleLoop` runs
- **THEN** it SHALL use runtime-owned decisions and helpers for provider turns,
  tool execution, transcript updates, approval waiting, typed child-runtime
  waiting, local steering, compaction, and event emission

#### Scenario: Simple loop preserves non-blocking child defaults

- **WHEN** `SimpleLoop` encounters child runtime state
- **THEN** it SHALL not force blocking child behavior by default
- **AND** any automatic waits it performs SHALL use the runtime’s explicit wait
  policy surface rather than implicit ad hoc holds

#### Scenario: Simple loop reacts to runtime advice with minimal policy

- **WHEN** the runtime exposes compaction advice or an unhandled doom-loop
  warning
- **THEN** `SimpleLoop` SHALL stay small and state-driven
- **AND** request compaction or one-shot steering through runtime decisions
  rather than reimplementing transcript or provider mechanics itself
