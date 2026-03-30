# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Brain Run Loop
`Brain.run(&project, &transport)` SHALL remain functional for simple scripting
and test use cases. Interactive clients and future remote clients SHALL prefer
the shared `BrainRuntime` boundary rather than depending on `BrainApi`.

#### Scenario: Brain.run still works
- **WHEN** `Brain.run(&project, &transport)` is called
- **THEN** it SHALL behave as before

#### Scenario: Interactive clients prefer BrainRuntime
- **WHEN** building an interactive client or a future remote runtime client
- **THEN** the preferred boundary SHALL be `BrainRuntime`
