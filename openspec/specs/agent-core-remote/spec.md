# agent-core-remote Specification

## Purpose
TBD - created by archiving change add-agent-core-remote. Update Purpose after archive.
## Requirements
### Requirement: Agent Core Remote Must Implement The Shared Core Boundary
The system MUST provide an `agent-core-remote` crate with an
`AgentCoreRemote` implementation of `agent_core::AgentCore`.

The remote implementation MUST connect to `agent-server` over the canonical
`/v1` API.

#### Scenario: Remote core connects to hosted server
- **WHEN** a caller constructs `AgentCoreRemote`
- **THEN** it SHALL connect to an `agent-server` base URL
- **AND** it SHALL fetch enough canonical metadata to satisfy synchronous
  shared-core methods locally

### Requirement: Remote Core Must Preserve Store Access Patterns
`AgentCoreRemote` MUST return proxy store handles that implement the same
composed `Store` trait surface as the native core.

#### Scenario: Consumer uses proxy store through shared boundary
- **WHEN** a caller uses `core.store()` on `AgentCoreRemote`
- **THEN** the returned store SHALL expose project, session, message,
  credential, and trajectory operations through the same traits used by the
  native core

### Requirement: Remote Core Must Stream Turns And Events Over HTTP
The remote core MUST preserve the streaming behavior of the shared boundary by
decoding canonical turn and event streams from `agent-server`.

#### Scenario: Turn stream is decoded from canonical NDJSON
- **WHEN** a caller starts a turn with `AgentCoreRemote`
- **THEN** the remote core SHALL decode `POST /v1/sessions/{id}/turns`
  responses as a stream of `CoreEvent` values

#### Scenario: Live events are decoded from canonical SSE
- **WHEN** a caller subscribes to `AgentCoreRemote`
- **THEN** the remote core SHALL decode `GET /v1/events` as a stream of
  `CoreEvent` values

