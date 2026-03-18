# brain-acp Specification

## ADDED Requirements

### Requirement: Standalone Mock ACP Surface

The system SHALL provide an ACP-compatible agent surface for `brain` as a
standalone proof of concept over stdio.

The first-stage ACP surface SHALL:

- support ACP over stdio for production use
- live in a dedicated `brain-acp` crate
- avoid dependencies on `brain-core`, `brain-server`, and other `brain-*`
  runtime crates
- be sufficient for manual validation with `acpx`, Nori, or another
  ACP-compatible client

#### Scenario: ACP editor client launches brain

- **WHEN** an ACP-compatible client launches `brain` in ACP mode over stdio
- **THEN** `brain` SHALL negotiate ACP capabilities and serve prompt turns as an ACP agent

#### Scenario: Mock ACP crate stays decoupled

- **WHEN** the first-stage ACP proof of concept is implemented
- **THEN** it SHALL not require `brain-core` or `brain-server` to function

### Requirement: Mock Session Lifecycle

The ACP surface SHALL support the broad mock session lifecycle exposed through
the enabled ACP SDK surface.

The first-stage implementation MAY use fixed or in-memory session data.

#### Scenario: New ACP session creates mock session

- **WHEN** a client sends `session/new` with a working directory
- **THEN** the ACP layer SHALL create and return a usable ACP session identifier

#### Scenario: Load ACP session replays mock history

- **WHEN** a client sends `session/load` for a known mock session
- **THEN** the ACP layer SHALL replay the visible conversation history through ACP `session/update` notifications before completing the load request

#### Scenario: ACP cancel stops turn

- **WHEN** a client sends `session/cancel` for an active session
- **THEN** the ACP layer SHALL stop the active mock turn and return an ACP-compatible cancelled stop reason

#### Scenario: ACP session list returns mock sessions

- **WHEN** a client sends `session/list`
- **THEN** the ACP layer SHALL return mock session metadata for the sessions it exposes

#### Scenario: ACP session resume returns active mock session

- **WHEN** the enabled ACP surface includes `session/resume` and a client resumes a known mock session
- **THEN** the ACP layer SHALL return a usable response without replaying prior history

#### Scenario: ACP session close releases mock session

- **WHEN** the enabled ACP surface includes `session/close` and a client closes a known mock session
- **THEN** the ACP layer SHALL cancel any active work for that session and mark it closed for future requests

### Requirement: Capability-Negotiated ACP Features

The ACP surface SHALL advertise only the optional ACP capabilities that the
mock implementation actually supports.

Optional ACP capabilities include, but are not limited to:

- session loading
- session modes
- session config options
- model selection
- session listing, forking, resuming, and closing
- extension methods and notifications

#### Scenario: Unsupported feature is not advertised

- **WHEN** the mock ACP implementation does not support an optional ACP feature
- **THEN** the ACP surface SHALL omit or reject that feature rather than claiming support

#### Scenario: Supported feature is advertised

- **WHEN** the mock ACP implementation supports an optional ACP feature
- **THEN** the ACP surface SHALL advertise it during initialization

### Requirement: Mock Prompt Streaming

The ACP surface SHALL stream prompt activity through ACP `session/update`
notifications using echoed or canned responses.

The first-stage implementation does not need to expose real tool execution,
filesystem access, terminal access, or runtime event mapping.

#### Scenario: Prompt turn streams mock output

- **WHEN** a client sends `session/prompt`
- **THEN** the ACP client SHALL receive ACP `session/update` notifications for the mocked agent response before the prompt request completes

#### Scenario: Prompt turn can be simple echo behavior

- **WHEN** the first-stage ACP implementation chooses echo behavior
- **THEN** the streamed response MAY include the client prompt content transformed into a simple canned reply

### Requirement: Mock ACP Control Surface

The mock ACP layer SHALL support non-runtime-backed control surfaces with fixed
or synthetic data where the protocol allows them.

This includes:

- session modes
- session config options
- extension methods and notifications
- optional model selection when enabled in the mock crate

#### Scenario: Session mode can be changed

- **WHEN** a client sends `session/set_mode` with a supported mock mode
- **THEN** the ACP layer SHALL update the session mode and return success

#### Scenario: Session config option can be changed

- **WHEN** a client sends `session/set_config_option` with a supported mock option value
- **THEN** the ACP layer SHALL update the session config state and return the new option set

#### Scenario: Extension method returns mock response

- **WHEN** a client sends a supported ACP extension method
- **THEN** the ACP layer SHALL return a deterministic mock response

#### Scenario: Session model can be changed

- **WHEN** the enabled ACP surface includes `session/set_model` and a client selects a supported mock model
- **THEN** the ACP layer SHALL update the session model and return success

### Requirement: Deferred Runtime Integration

The first-stage ACP proof of concept SHALL defer integration with the real
`brain` runtime to a later change.

Deferred work includes:

- `brain-core` integration
- real persisted session mapping
- filesystem and terminal client capability integration
- approval flows
- full event-model alignment

#### Scenario: Runtime-backed features are deferred

- **WHEN** a runtime-backed ACP feature is not yet implemented in the mock stage
- **THEN** the ACP layer SHALL omit or reject it rather than simulate deep integration with `brain`

### Requirement: Mock Client-Owned ACP Requests

The mock ACP layer SHALL be able to exercise client-owned ACP request methods
through deterministic `mock:`-prefixed prompt commands.

These flows MAY be exposed through prompt commands rather than through a real
tool system, but they SHALL stream their synthetic results through ACP
tool-call updates so ACP clients can render them like normal agent actions.

#### Scenario: Mock prompt triggers client file read

- **WHEN** a client sends a supported `mock:` prompt command for file reading
- **THEN** the ACP layer SHALL call the client `fs/read_text_file` method and stream a deterministic synthetic result back through ACP tool-call updates

#### Scenario: Mock prompt command omits optional arguments

- **WHEN** a client sends a supported `mock:` prompt command without optional arguments
- **THEN** the ACP layer SHALL choose deterministic defaults so the mock interaction remains runnable without additional input

#### Scenario: Mock prompt triggers synthetic file write

- **WHEN** a client sends a supported `mock:` prompt command for file writing
- **THEN** the ACP layer SHALL stream a deterministic synthetic write result back through ACP tool-call updates without requiring a real filesystem side effect

#### Scenario: Mock prompt triggers permission request

- **WHEN** a client sends a supported `mock:` prompt command for permission testing
- **THEN** the ACP layer SHALL call the client `session/request_permission` method and stream the selected outcome back through ACP tool-call updates

#### Scenario: Mock prompt triggers terminal round trip

- **WHEN** a client sends a supported `mock:` prompt command for terminal testing
- **THEN** the ACP layer SHALL call the client terminal methods needed to create, inspect, and finish a terminal interaction and stream the result back through ACP tool-call updates

#### Scenario: Mock prompt triggers explicit mock planning

- **WHEN** a client sends `mock:create-plan`
- **THEN** the ACP layer SHALL emit a deterministic synthetic plan update sequence without requiring a normal prompt turn to create one

#### Scenario: Mock prompt triggers reasoning chunks

- **WHEN** a client sends `mock:think <topic>` or `mock:think`
- **THEN** the ACP layer SHALL emit deterministic ACP reasoning updates through `AgentThoughtChunk` before completing the turn

#### Scenario: Mock prompt triggers higher-level tool kinds

- **WHEN** a client sends supported `mock:` prompt commands for search or fetch behavior
- **THEN** the ACP layer SHALL emit deterministic `ToolCall` and `ToolCallUpdate` flows using the corresponding ACP tool kinds

#### Scenario: Mock prompt triggers synthetic patch-style file operations

- **WHEN** a client sends supported `mock:` prompt commands for edit, delete, or move behavior
- **THEN** the ACP layer SHALL emit deterministic patch-style ACP tool updates without mutating real files
