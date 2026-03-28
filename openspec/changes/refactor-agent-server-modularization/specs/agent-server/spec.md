## ADDED Requirements

### Requirement: Canonical HTTP Internals Are Organized By Route Family

The canonical `agent-server` `/v1` implementation SHALL keep route
registration separate from large route-family adapters and shared HTTP error
translation.

#### Scenario: Canonical chat completion adaptation is isolated

- **WHEN** the server implements `/v1/chat/completions`
- **THEN** the OpenAI-compatible request and response adaptation SHALL live in a
  dedicated canonical HTTP module
- **AND** it SHALL NOT share a catch-all implementation file with unrelated
  credential CRUD handlers

#### Scenario: Canonical turn transport and error translation are reusable

- **WHEN** the server implements canonical turn and credential endpoints
- **THEN** transport-specific turn handlers and shared error-to-response
  translation SHALL live in focused canonical HTTP modules
- **AND** route registration MAY remain centralized for route inventory clarity
