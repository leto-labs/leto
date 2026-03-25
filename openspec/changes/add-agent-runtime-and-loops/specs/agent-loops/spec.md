## ADDED Requirements

### Requirement: Agent Loops Crate Provides Strategy Implementations

The system SHALL provide an `agent-loops` crate containing loop strategies built
on the standalone `agent-runtime` crate.

The crate SHALL NOT depend on any `brain-*` crate.

#### Scenario: Loop crate depends on runtime only

- **WHEN** a caller depends on `agent-loops`
- **THEN** the crate SHALL resolve against `agent-runtime` and the shared
  provider types rather than legacy loop/runtime crates

### Requirement: Agent Loops Ships A SimpleLoop Reference Strategy

The crate SHALL provide a `SimpleLoop` strategy that performs provider
inference, executes requested tools through the runtime, appends tool results,
and re-enters inference until completion or a configured limit is reached.

#### Scenario: Simple loop finishes after plain assistant output

- **WHEN** the provider returns assistant output without tool calls
- **THEN** `SimpleLoop` SHALL finish the turn successfully

#### Scenario: Simple loop re-enters after tool execution

- **WHEN** the provider requests a tool call
- **THEN** `SimpleLoop` SHALL execute the tool through the runtime
- **AND** append the tool result
- **AND** continue the turn with another provider step

### Requirement: SimpleLoop Uses Reusable Runtime Helpers

`SimpleLoop` SHALL use the public reusable runtime surface rather than private
provider orchestration or transcript mutation shortcuts.

#### Scenario: Simple loop stays within runtime boundary

- **WHEN** `SimpleLoop` runs
- **THEN** it SHALL use runtime-owned decisions and helpers for provider turns,
  tool execution, transcript updates, approval waiting, typed child-runtime
  waiting, and event emission

#### Scenario: Simple loop preserves non-blocking child defaults

- **WHEN** `SimpleLoop` encounters child runtime state
- **THEN** it SHALL not force blocking child behavior by default
- **AND** any automatic waits it performs SHALL use the runtime’s explicit wait
  policy surface rather than implicit ad hoc holds
