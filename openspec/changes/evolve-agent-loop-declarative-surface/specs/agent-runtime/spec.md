## ADDED Requirements

### Requirement: Agent Runtime Exposes Bounded Declarative Advanced Loop Effects

The runtime SHALL expose runtime-native loop effects for isolated provider
subcalls and safe transcript edits without requiring loops to mutate runtime
state directly.

#### Scenario: Loop runs a tagged provider subcall

- **WHEN** a loop requests a provider subcall with a stable purpose label
- **THEN** the runtime SHALL execute that subcall without mutating the outer
  transcript
- **AND** record a typed subcall result in loop-visible session state

#### Scenario: Loop rewrites or appends transcript through runtime effects

- **WHEN** a loop requests a concrete transcript rewrite or transcript append
- **THEN** the runtime SHALL apply that change itself rather than exposing raw
  transcript mutation to the loop
- **AND** record a typed operation result in loop-visible session state

### Requirement: Agent Runtime Records Recent Operation Results For Loop Re-entry

The runtime SHALL expose typed recent operation results so advanced loops can
branch across ticks without plan-level variables or nested workflow objects.

#### Scenario: Loop reads prior runtime-native operation results

- **WHEN** a loop re-enters after a prior subcall or transcript edit
- **THEN** it SHALL be able to inspect the most recent typed operation results
  through session state
- **AND** use that state to decide the next single runtime-native effect
