# brain-core Delta Spec

## ADDED Requirements

### Requirement: Compatibility Adapters Layer On BrainRuntime
The system SHALL treat any future OpenCode-compatible server surface as an
adapter layered on top of Brain's canonical runtime and store abstractions.

That adapter SHALL translate Brain semantics outward to the compatibility API
rather than introducing a parallel canonical model centered on OpenCode
terminology.

#### Scenario: Compatibility architecture is implemented later
- **WHEN** a future compatibility adapter is added
- **THEN** it SHALL be built on top of Brain runtime and store semantics
- **AND** it SHALL not replace `BrainRuntime` as the canonical boundary

### Requirement: Canonical Runtime Traits Stay OpenCode-Agnostic
The canonical runtime architecture SHALL NOT absorb OpenCode-specific domain
concepts merely to simplify the compatibility layer.

Project/workspace mapping, session mapping, provider/config mapping, and
credential mapping SHALL be expressed through Brain's existing runtime and
store abstractions rather than by reshaping those abstractions around the
compatibility consumer.

#### Scenario: Contributor proposes adding OpenCode-specific runtime concepts
- **WHEN** a contributor proposes changing runtime traits mainly to match the
  OpenCode-compatible API
- **THEN** that proposal SHALL be considered a violation of the planned
  layering
- **AND** the compatibility layer SHALL remain responsible for translation
