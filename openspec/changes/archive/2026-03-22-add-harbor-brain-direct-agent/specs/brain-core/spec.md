## ADDED Requirements

### Requirement: Brain Core Emits Native ATIF Completion Events

When ATIF emission is enabled in the effective agent config, `brain-core` SHALL emit native ATIF completion records on the normal turn event stream.

These records SHALL be additive to the operational runtime stream and SHALL NOT
replace token, tool, progress, or error events.

#### Scenario: Turn emits ATIF completion records

- **WHEN** a turn runs with ATIF emission enabled
- **THEN** the turn stream SHALL include `Event::Atif(...)` completion records
- **AND** the normal operational events SHALL still be emitted

### Requirement: TurnDone Remains Terminal

`TurnDone` SHALL remain the terminal event of a successfully completed turn,
including when ATIF emission is enabled.

#### Scenario: ATIF completion precedes TurnDone

- **WHEN** a turn completes with ATIF emission enabled
- **THEN** any ATIF completion records for that turn SHALL be emitted before
  `TurnDone`
- **AND** `TurnDone` SHALL be emitted last

### Requirement: ATIF Transcript State Preserves Historical Metadata

`brain-core` SHALL preserve historical ATIF transcript metadata across turns rather than rebuilding prior assistant steps from the current turn config.

#### Scenario: Model change does not rewrite prior ATIF steps

- **WHEN** a session changes model between turns
- **THEN** newly appended assistant ATIF steps SHALL use the new model metadata
- **AND** previously persisted ATIF steps SHALL preserve their original model
  metadata
