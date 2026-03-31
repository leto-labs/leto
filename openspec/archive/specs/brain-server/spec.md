# brain-server Specification

## Purpose
Legacy server-side event and status infrastructure for `brain`. The crate is
currently kept out of the active workspace while future remote-runtime
architecture remains deferred to the active server-architecture change.

## Requirements

### Requirement: ServerEvent
The system SHALL define a `ServerEvent` struct that wraps a brain `Event` with
session context.

The event wrapper SHALL include:

- `session_id: Ulid`
- `event: Event`
- `timestamp: DateTime<Utc>`

#### Scenario: Event with session context
- **WHEN** a turn emits a `Token { delta }` event for a session
- **THEN** the `ServerEvent` SHALL carry that session ID
- **AND** include the wrapped event and timestamp

### Requirement: EventBus
The system SHALL provide an `EventBus` backed by
`tokio::sync::broadcast` that distributes `ServerEvent` instances to live
subscribers.

The bus SHALL expose:

- `subscribe() -> broadcast::Receiver<ServerEvent>`
- `publish(event: ServerEvent)`

#### Scenario: Multiple subscribers receive the same event
- **WHEN** two callers subscribe to the event bus
- **AND** a `ServerEvent` is published
- **THEN** both subscribers SHALL receive that event

#### Scenario: Slow subscriber does not block the bus
- **WHEN** one subscriber falls behind and lags the broadcast channel
- **THEN** the bus SHALL continue functioning for other subscribers
- **AND** the lagging subscriber MAY miss events

### Requirement: ServerStatus
The system SHALL define a `ServerStatus` struct for reporting basic server
state.

The status SHALL include:

- `providers: Vec<ProviderInfo>`
- `tools: Vec<String>`
- `active_sessions: usize`
- `active_turns: Vec<Ulid>`

#### Scenario: Status reports current runtime inventory
- **WHEN** server status is queried
- **THEN** it SHALL report provider metadata, tool names, session count, and active turns

### Requirement: Legacy Server Surface Stays Out Of The Active Workspace
The current repository SHALL treat `brain-server` as a legacy crate that is
kept out of the active workspace while the future remote-runtime architecture
is still being redesigned.

Legacy `BrainApi`, `BrainServer`, and HTTP transport code MAY remain in the
crate as implementation details, but they SHALL NOT be treated as the current
preferred application boundary for CLI or ACP clients.

#### Scenario: Contributor evaluates current client architecture
- **WHEN** a contributor reviews the current local client surfaces
- **THEN** `agent-cli` and `agent-acp` SHALL be understood to use `AgentCore`
- **AND** `brain-server` SHALL be understood as deferred legacy infrastructure
