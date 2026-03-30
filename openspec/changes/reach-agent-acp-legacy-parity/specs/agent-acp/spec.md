## ADDED Requirements

### Requirement: Dedicated Agent ACP Binaries

The system SHALL expose the real ACP surface through a direct `agent-acp`
binary identity.

The migration plan SHALL also preserve an explicit mock strategy, either as a
direct `agent-acp-mock` binary or as an explicitly documented replacement that
keeps the current repo validation lane intact.

#### Scenario: Real ACP backend is launched directly

- **WHEN** a user or ACP client launches the real ACP backend
- **THEN** it SHALL be possible to launch it through a direct `agent-acp`
  binary identity rather than only through `agent acp`

#### Scenario: Mock ACP validation path remains explicit

- **WHEN** repo validation or interoperability testing needs the mock ACP
  backend
- **THEN** the repo SHALL expose an explicit supported mock launch path rather
  than silently removing that lane

### Requirement: Real ACP Session Model Compatibility

The real `agent-acp` backend SHALL preserve the session-model compatibility
surface currently used by repo integrations.

This SHALL include:

- reporting effective session model state from real session/runtime data
- supporting session model mutation through either `session/set_model` or a
  documented replacement updated together with all current consumers

#### Scenario: Harbor-compatible session model inspection remains available

- **WHEN** an ACP client loads or creates a session that participates in the
  current Harbor workflow
- **THEN** it SHALL be able to inspect the effective session model state from
  the real backend

#### Scenario: Harbor-compatible session model mutation remains available

- **WHEN** the current Harbor workflow changes the session model
- **THEN** the real ACP backend and its repo consumers SHALL continue to work
  together through a supported compatibility path

### Requirement: Real Backend Can Bridge File Reads And Writes Through ACP Clients

The real `agent-acp` backend SHALL be able to route file reads and writes
through ACP client-owned filesystem capabilities when running under a client
that provides them.

This path SHALL allow containerized ACP clients such as Harbor to keep file
edits inside the client-managed task workspace rather than writing only to the
backend host filesystem.

#### Scenario: Harbor-backed ACP file operations land in the task workspace

- **WHEN** the real `agent-acp` backend is launched through Harbor's ACP client
- **AND** the model uses `file_write` or `file_read` with a relative path
- **THEN** the file operation SHALL be sent through the ACP client bridge
- **AND** the resolved path SHALL be rooted in the ACP session cwd exposed by
  the client
