# agent-server Specification

## Purpose
TBD - created by archiving change add-agent-server-stubs. Update Purpose after archive.
## Requirements
### Requirement: Agent Server Must Host The Shared Core Boundary
The system MUST provide an `agent-server` crate that hosts `Arc<dyn AgentCore>`
rather than a separate client API trait.

#### Scenario: Hosted server uses shared core
- **WHEN** `agent-server` is constructed
- **THEN** it SHALL accept a shared `AgentCore` implementation
- **AND** the hosted boundary SHALL remain aligned with the same core interface
  used by other consumers

### Requirement: Canonical API Is Versioned Under `/v1`
The canonical hosted API MUST live under `/v1`.

The canonical API MUST include the surfaces needed for remote core parity,
including:

- health and status
- project, session, message, credential, and trajectory access
- project-root lookup and project resolution
- provider and model introspection
- request-scoped turn execution
- turn cancellation
- live runtime-bus observation

#### Scenario: Canonical turn streaming preserves request scope
- **WHEN** a caller starts a turn through `POST /v1/sessions/{id}/turns`
- **THEN** the server SHALL return a chunked NDJSON stream of core events for
  that turn

#### Scenario: Canonical runtime bus is live over SSE
- **WHEN** a caller subscribes to `GET /v1/events`
- **THEN** the server SHALL expose the live shared core event bus over SSE

#### Scenario: Canonical API covers full store parity
- **WHEN** a remote caller uses `AgentCoreRemote`
- **THEN** the canonical `/v1` API SHALL support the full composed `Store`
  interface without requiring in-process access

### Requirement: OpenCode Compatibility Surface Is Secondary
The system MUST expose an OpenCode-compatible secondary surface under
`/v1/compat/opencode`.

That compatibility surface MUST remain separate from the canonical API and MUST
target the session-oriented app API rather than unrelated upstream route
families.

#### Scenario: Compatibility routes are version-scoped and namespaced
- **WHEN** a caller uses the OpenCode-compatible surface
- **THEN** it SHALL do so under `/v1/compat/opencode`
- **AND** the canonical `/v1` API SHALL remain the primary hosted contract

### Requirement: Compatibility Reads May Be Real While Writes Stay Stubbed
The first compatibility pass MUST implement low-risk read flows where the
mapping is obvious and MUST return explicit `501 not_implemented` responses for
unresolved writes or semantics-heavy flows.

#### Scenario: Low-risk compat read is available
- **WHEN** a caller uses compatibility health, event, or basic list/read
  endpoints
- **THEN** the server MAY return real mapped data

#### Scenario: Unresolved compat write returns explicit stub response
- **WHEN** a caller uses a compatibility route whose semantics are not yet
  mapped cleanly
- **THEN** the server SHALL return a `501 not_implemented` response
- **AND** that response SHALL be machine-readable

### Requirement: Canonical Protocol Is Shared With The Remote Core
The system MUST share the canonical `/v1` wire contract between `agent-server`
and `AgentCoreRemote`.

#### Scenario: Server and remote client use one canonical protocol definition
- **WHEN** the canonical `/v1` request or response shapes are compiled
- **THEN** `agent-server` and `agent-core-remote` SHALL use the same DTO
  definitions

