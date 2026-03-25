# agent-server Delta Spec

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Canonical Protocol Is Shared With The Remote Core
The system MUST share the canonical `/v1` wire contract between `agent-server`
and `AgentCoreRemote`.

#### Scenario: Server and remote client use one canonical protocol definition
- **WHEN** the canonical `/v1` request or response shapes are compiled
- **THEN** `agent-server` and `agent-core-remote` SHALL use the same DTO
  definitions
