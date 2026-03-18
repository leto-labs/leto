# brain-acp Specification

## ADDED Requirements

### Requirement: First-Class ACP Surface

The system SHALL provide an ACP-compatible agent surface for `brain` as a
first-class interactive client surface alongside `BrainServer`.

The ACP surface SHALL:

- support ACP over stdio for production use
- be designed around the same runtime concepts as `BrainServer`
- not depend on a thin HTTP bridge as its defining architecture

#### Scenario: ACP editor client launches brain

- **WHEN** an ACP-compatible client launches `brain` in ACP mode over stdio
- **THEN** `brain` SHALL negotiate ACP capabilities and serve prompt turns as an ACP agent

#### Scenario: BrainServer remains primary too

- **WHEN** `brain` also exposes `BrainServer`
- **THEN** ACP SHALL coexist with `BrainServer` as a parallel primary surface rather than replacing it

### Requirement: ACP Session Mapping

The ACP surface SHALL map ACP session lifecycle onto `brain` session lifecycle.

At minimum, the ACP surface SHALL support:

- creating a new session
- loading an existing session when the runtime supports it
- prompting within a session
- cancelling an active turn

#### Scenario: New ACP session creates brain session

- **WHEN** a client sends `session/new` with a working directory
- **THEN** `brain` SHALL create a corresponding runtime session scoped to that context

#### Scenario: Load ACP session replays history

- **WHEN** a client sends `session/load` for a persisted session
- **THEN** `brain` SHALL replay the visible conversation history through ACP `session/update` notifications before completing the load request

#### Scenario: ACP cancel stops turn

- **WHEN** a client sends `session/cancel` for an active session
- **THEN** `brain` SHALL cancel the active turn and return an ACP-compatible cancelled stop reason

### Requirement: Capability-Negotiated ACP Features

The ACP surface SHALL advertise optional ACP capabilities only when they are
backed by real runtime support.

Optional ACP capabilities include, but are not limited to:

- session loading
- session modes
- session config options
- model selection
- session listing, forking, resuming, and closing

#### Scenario: Unsupported feature is not advertised

- **WHEN** `brain` does not support an optional ACP feature
- **THEN** the ACP surface SHALL omit or reject that feature rather than claiming support

#### Scenario: Supported feature is advertised

- **WHEN** `brain` supports an optional ACP feature
- **THEN** the ACP surface SHALL advertise it during initialization

### Requirement: ACP Event Mapping

The ACP surface SHALL map `brain` turn activity into ACP session updates without
loss of essential user-visible semantics.

The mapping SHALL cover:

- agent message streaming
- plan or status updates when the runtime supports them
- tool call lifecycle
- permission requests and outcomes
- terminal-related updates when the client supports them
- stop reasons and errors

#### Scenario: Prompt turn streams output and tools

- **WHEN** a `brain` turn emits message chunks and tool activity
- **THEN** the ACP client SHALL receive corresponding ACP `session/update` notifications for the same turn

#### Scenario: Permission request becomes ACP approval UI

- **WHEN** `brain` requires user approval for a tool action
- **THEN** the ACP surface SHALL issue ACP permission requests so the client can present native approval UI

### Requirement: ACP Client Capability Integration

The ACP surface SHALL use ACP client-owned capabilities when they are available,
and degrade cleanly when they are not.

These client-owned capabilities include:

- filesystem read and write methods
- terminal creation, output, and lifecycle methods
- native approval UX via permission requests

#### Scenario: Client filesystem support is present

- **WHEN** the ACP client advertises filesystem read and write capabilities
- **THEN** `brain` SHALL be able to use ACP client filesystem methods for session-scoped file operations

#### Scenario: Client terminal support is present

- **WHEN** the ACP client advertises terminal support
- **THEN** `brain` SHALL be able to execute session-scoped terminal actions through ACP client terminal methods

#### Scenario: Client capability is absent

- **WHEN** the ACP client does not advertise an optional client capability
- **THEN** `brain` SHALL not call that ACP method and SHALL continue with supported behavior only
