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

The canonical API SHALL be fully implemented for every exposed route. The
canonical surface SHALL NOT rely on placeholder or `501 not_implemented`
behavior for routes that are present.

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
target OpenCode release `v1.3.2` as its compatibility baseline.

#### Scenario: Compatibility routes are version-scoped and namespaced
- **WHEN** a caller uses the OpenCode-compatible surface
- **THEN** it SHALL do so under `/v1/compat/opencode`
- **AND** the canonical `/v1` API SHALL remain the primary hosted contract

## ADDED Requirements

### Requirement: OpenCode Compatibility Must Cover The Full OpenAPI Contract
The compatibility surface MUST cover the full OpenCode route inventory declared
in the local `openapi/opencode.json` contract artifact.

This includes the full set of route groups exposed by the OpenCode server for
that release, not only session or turn endpoints.

#### Scenario: Every declared compat route exists
- **WHEN** the local `openapi/opencode.json` artifact is used as the
  compatibility contract
- **THEN** every declared path and method SHALL exist under
  `/v1/compat/opencode`

#### Scenario: Declared compat route is not left as a stub
- **WHEN** an operation exists in the OpenCode compatibility contract
- **THEN** `agent-server` SHALL implement it with real behavior or a real
  compat-local adapter
- **AND** SHALL NOT respond with `501 not_implemented`

### Requirement: Compatibility `/doc` Must Reflect The Implemented Surface
The compatibility endpoint `/v1/compat/opencode/doc` MUST be generated from the
implemented compatibility surface rather than remaining a static placeholder.

The generated document SHALL match the local `openapi/opencode.json` artifact
for the full documented interface, except for the `/v1/compat/opencode` path
prefix.

#### Scenario: Generated compat spec matches the local contract
- **WHEN** `/v1/compat/opencode/doc` is compared with
  `openapi/opencode.json`
- **THEN** path and method inventory SHALL match after accounting for the
  `/v1/compat/opencode` prefix
- **AND** operation identifiers SHALL match
- **AND** request bodies SHALL match
- **AND** response status codes and content types SHALL match
- **AND** response schemas and referenced component schemas SHALL match
- **AND** any remaining differences SHALL be limited to explicitly normalized
  non-semantic metadata such as ordering

#### Scenario: Generated compat spec is not a static passthrough
- **WHEN** `/v1/compat/opencode/doc` is served
- **THEN** it SHALL be produced entirely from Rust-side compatibility metadata
- **AND** runtime server code SHALL NOT import or embed
  `openapi/opencode.json`
- **AND** the local contract artifact SHALL be used only in parity tests

### Requirement: Compatibility Surface Must Support Browser-Facing Clients
The compatibility surface MUST support browser-facing and Tauri-style clients
well enough to enable later OpenCode UI validation.

This includes practical origin handling and streaming behavior, not just route
existence.

#### Scenario: Compatibility preflight succeeds for allowed UI origins
- **WHEN** a browser or Tauri-style client issues a CORS preflight request to
  `/v1/compat/opencode`
- **THEN** the server SHALL return headers that allow the configured request to
  proceed

#### Scenario: Compatibility event streams stay open
- **WHEN** a caller subscribes to OpenCode-compatible SSE endpoints
- **THEN** the server SHALL emit an initial connected event
- **AND** keep the stream alive with heartbeat-compatible behavior
