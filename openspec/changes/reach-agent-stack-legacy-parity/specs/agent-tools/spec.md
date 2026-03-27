## ADDED Requirements

### Requirement: Agent Tools Provide Terminal-Session Parity

The `agent-tools` crate SHALL provide a terminal-session tool surface that
restores the user-visible behavior of the legacy terminal session tooling on the
refactored stack.

This SHALL include:

- stable terminal-session identity across calls
- command execution against an existing session
- explicit lifecycle operations such as close or equivalent cleanup
- typed contracts suitable for default local core assembly

#### Scenario: Caller reuses a terminal session across multiple tool calls

- **WHEN** a model or caller opens a terminal session and later issues more
  terminal operations against that same session
- **THEN** `agent-tools` SHALL preserve the session identity and route those
  operations to the same live terminal context

#### Scenario: Terminal-session tool definitions are available in default assembly

- **WHEN** a caller builds the default native tool registry
- **THEN** the registry SHALL expose the terminal-session parity surface as part
  of the available tool definitions
