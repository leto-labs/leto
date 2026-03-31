## MODIFIED Requirements

### Requirement: ACP Compatibility Launch Path Remains Available

The system SHALL keep `agent acp` as a supported local launch path for the
runtime-backed ACP stdio server.

The `agent-acp` crate SHALL continue to expose a direct stdio entrypoint for
repo-owned binaries and tests.

The repo SHALL also expose:

- a direct `agent-acp` binary identity for repo-owned ACP integrations
- an explicit `agent-acp-mock` launch path for ACP smoke validation

#### Scenario: Existing agent ACP command starts the backend

- **WHEN** a user launches `agent acp`
- **THEN** the ACP stdio server SHALL start successfully against the embedded
  `AgentCore`

#### Scenario: Repo-owned code can call the stdio entrypoint directly

- **WHEN** a repo-owned binary or test needs to start the real ACP backend
- **THEN** it SHALL be able to call the exported `agent_acp::run_stdio(...)`
  entrypoint

#### Scenario: Direct agent-acp binary is available

- **WHEN** a repo-owned integration such as Harbor needs the real ACP backend
- **THEN** it SHALL be able to launch a direct `agent-acp` binary identity

#### Scenario: ACP smoke validation keeps an explicit mock lane

- **WHEN** the repo runs the `acpx` compatibility harness
- **THEN** it SHALL be able to launch an explicit `agent-acp-mock` backend
  without requiring external model credentials
