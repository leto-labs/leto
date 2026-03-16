# brain-transports Delta Spec

## MODIFIED Requirements

### Requirement: Transport Trait
The Transport trait SHALL remain unchanged. It continues to serve `Brain.run()` for simple use cases. However, interactive clients (TUI, ACP) SHALL prefer using `BrainApi` directly rather than implementing Transport.

#### Scenario: Transport still used by Brain.run
- **WHEN** `Brain.run(&transport)` is called
- **THEN** the Transport trait is used as before

#### Scenario: TUI does not implement Transport
- **WHEN** the TUI is built
- **THEN** it SHALL use `BrainApi` to communicate with the engine, not implement the Transport trait


## MODIFIED Requirements

### Requirement: Transport Trait
The Transport trait SHALL remain unchanged. It continues to serve `Brain.run()` for simple use cases. However, interactive clients (TUI, ACP) SHALL prefer using `BrainApi` directly rather than implementing Transport.

#### Scenario: Transport still used by Brain.run
- **WHEN** `Brain.run(&transport)` is called
- **THEN** the Transport trait is used as before

#### Scenario: TUI does not implement Transport
- **WHEN** the TUI is built
- **THEN** it SHALL use `BrainApi` to communicate with the engine, not implement the Transport trait
