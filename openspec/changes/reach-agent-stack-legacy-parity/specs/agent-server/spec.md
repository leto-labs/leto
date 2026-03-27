## ADDED Requirements

### Requirement: Compatibility Auth Routes Use Real Provider-Backed Behavior

The `agent-server` compatibility surface SHALL implement documented provider and
MCP auth routes with real behavior rather than placeholder URLs or synthetic
credentials.

#### Scenario: Compatibility provider authorize route returns a real auth start

- **WHEN** a caller starts a provider auth flow through the compatibility
  surface
- **THEN** the server SHALL return a real provider-backed auth initiation
  result compatible with the configured auth flow

#### Scenario: Compatibility provider callback persists real credentials

- **WHEN** a caller completes a provider auth callback through the
  compatibility surface
- **THEN** the server SHALL resolve and persist real credentials rather than
  storing placeholder token values

### Requirement: Compatibility PTY Routes Use Real Runtime-Backed Behavior

The `agent-server` compatibility surface SHALL implement documented PTY routes
with real runtime-backed behavior rather than static success placeholders.

#### Scenario: Compatibility PTY connect attaches to a live PTY session

- **WHEN** a caller connects to a PTY through the compatibility surface
- **THEN** the server SHALL attach the request to a real live PTY or terminal
  session managed by the refactored stack

#### Scenario: Compatibility PTY operations reflect live session state

- **WHEN** a caller performs PTY reads, writes, or lifecycle operations through
  the compatibility surface
- **THEN** the server SHALL reflect the state of the underlying live PTY
  session rather than returning placeholder success responses
