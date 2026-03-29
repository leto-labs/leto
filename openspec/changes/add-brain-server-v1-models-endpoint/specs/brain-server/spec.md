## ADDED Requirements

### Requirement: Legacy Brain Server Exposes A V1 Models Compatibility Route

The legacy `brain-server` HTTP surface SHALL expose a `GET /v1/models`
endpoint for compatibility with clients that discover model inventory through
an OpenAI-style route.

#### Scenario: V1 models returns provider inventory

- **WHEN** a caller requests `GET /v1/models`
- **THEN** the server SHALL respond with `200 OK`
- **AND** return the same provider inventory exposed by `GET /providers`
