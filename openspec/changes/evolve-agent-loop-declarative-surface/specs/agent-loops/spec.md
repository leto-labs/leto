## ADDED Requirements

### Requirement: Agent Loops Compose Advanced Runtime Effects Across Ticks

Loop strategies SHALL be able to compose richer runtime-native effects across
multiple ticks without requiring a separate programmable-loop architecture.

#### Scenario: Advanced loop chains subcall and transcript rewrite

- **WHEN** a loop first requests a tagged subcall and later sees that subcall
  result in session state
- **THEN** it SHALL be able to request a subsequent transcript rewrite or
  transcript append on a later tick
- **AND** do so by returning one concrete runtime-native effect at a time

### Requirement: Simple Loops Remain A Small-Subset User Of The Same Surface

The addition of advanced runtime-native effects SHALL NOT require simple loops
to adopt nested plans or workflow-style control flow.

#### Scenario: Simple loop continues using a bounded subset

- **WHEN** `SimpleLoop` runs on top of the widened runtime surface
- **THEN** it SHALL be able to continue using the small existing subset of
  provider, tool, wait, steering, and compaction decisions
- **AND** remain a declarative strategy rather than a separate loop
  architecture
