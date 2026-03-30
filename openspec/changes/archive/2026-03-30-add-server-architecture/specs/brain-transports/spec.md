# brain-transports Delta Spec

## MODIFIED Requirements

### Requirement: Transport Trait
The `Transport` trait SHALL remain unchanged and continue to serve
`Brain.run(...)` for simple use cases. Interactive clients and future remote
clients SHALL prefer the shared `BrainRuntime` boundary rather than treating
`Transport` as the primary interactive integration point.

#### Scenario: Transport still used by Brain.run
- **WHEN** `Brain.run(&transport)` is called
- **THEN** the `Transport` trait is used as before

#### Scenario: Interactive clients prefer BrainRuntime
- **WHEN** an interactive client or future remote runtime client is built
- **THEN** it SHALL use `BrainRuntime`-oriented integration rather than implementing `Transport`
