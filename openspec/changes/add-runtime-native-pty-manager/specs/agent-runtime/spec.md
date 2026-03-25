## ADDED Requirements

### Requirement: Agent Runtime Manages PTY Sessions As First-Class Runtime Resources

The runtime SHALL manage PTY sessions as explicit runtime resources with stable
identity, lifecycle, and queryable state.

#### Scenario: Open and inspect a PTY session

- **WHEN** a caller opens a PTY through the runtime
- **THEN** the runtime SHALL assign a stable PTY identifier
- **AND** retain PTY metadata and status in runtime-managed state
- **AND** allow that PTY to be listed or fetched by id later

### Requirement: Agent Runtime Exposes Typed PTY Control Operations

The runtime SHALL expose typed PTY operations rather than hiding PTY execution
behind generic tool payloads.

#### Scenario: Execute and capture a PTY session

- **WHEN** a caller writes input or executes commands inside a PTY
- **THEN** the runtime SHALL return structured execution results and PTY
  snapshots
- **AND** support explicit capture, resize, interrupt, background, and close
  operations against the same PTY id

### Requirement: PTY Events Can Be Subscribed And Safely Promoted

The runtime SHALL expose PTY event subscriptions and allow explicitly promoted
PTY events to become safe-boundary `developer` transcript messages.

#### Scenario: Subscribe to PTY output and promote it

- **WHEN** a runtime explicitly subscribes to PTY output with developer-message
  promotion enabled
- **THEN** matching PTY events SHALL remain observable as runtime events
- **AND** the runtime SHALL queue model-visible `developer` messages only at
  safe boundaries
- **AND** PTY events SHALL NOT be auto-injected into transcript without an
  explicit subscription
