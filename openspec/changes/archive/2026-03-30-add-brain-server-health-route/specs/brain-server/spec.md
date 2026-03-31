## ADDED Requirements

### Requirement: Legacy Brain Server Exposes A Lightweight Health Probe

The legacy `brain-server` HTTP surface SHALL expose a lightweight liveness
endpoint separate from the richer `/status` response.

#### Scenario: Health probe returns ok

- **WHEN** a caller requests `GET /health`
- **THEN** the server SHALL respond with `200 OK`
- **AND** return a JSON body indicating the server is healthy
