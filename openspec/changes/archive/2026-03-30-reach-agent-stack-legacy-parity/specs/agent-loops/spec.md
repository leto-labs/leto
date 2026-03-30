## ADDED Requirements

### Requirement: Agent Loops Provide Advanced Legacy-Parity Strategies

The `agent-loops` crate SHALL provide advanced loop strategies that restore the
legacy behaviors previously exposed through `robust`, `terminus2`, and
`terminus-kira`.

The refactored implementations MAY use v2-native runtime effects and state, but
they SHALL preserve the externally observable loop behavior required by those
legacy strategies.

#### Scenario: Robust-style loop handles iterative tool use with recovery behavior

- **WHEN** a session uses the robust-parity loop on the refactored runtime
- **THEN** it SHALL support the same class of multi-step tool and retry
  behavior expected from the legacy robust loop

#### Scenario: Terminus-style loop performs advanced terminal-oriented workflows

- **WHEN** a session uses a terminus-parity loop on the refactored runtime
- **THEN** it SHALL support the advanced multi-step terminal-oriented behavior
  expected from the corresponding legacy terminus strategy

### Requirement: Advanced Agent Loops Remain Runtime-Native

Advanced parity loops SHALL be implemented as `agent-runtime` strategies rather
than legacy adapters.

#### Scenario: Advanced parity loops depend on refactored runtime surfaces

- **WHEN** a caller depends on advanced `agent-loops` strategies
- **THEN** those strategies SHALL execute through runtime-native loop effects,
  PTY/session surfaces, and transcript operations rather than invoking legacy
  `brain-*` loop implementations
