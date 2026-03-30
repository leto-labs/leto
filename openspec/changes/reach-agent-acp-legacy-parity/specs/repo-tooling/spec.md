## ADDED Requirements

### Requirement: Repo-Owned ACP Integrations Use Agent ACP Entrypoints

Repo-owned ACP launchers, Harbor agents, and compatibility scripts SHALL target
the supported `agent-acp` real and mock entrypoints rather than `brain-acp`
identities.

#### Scenario: Harbor launches the real Agent ACP backend

- **WHEN** Harbor launches the repo's ACP backend
- **THEN** it SHALL use the supported `agent-acp` entrypoint rather than
  `brain-acp`

#### Scenario: Repo compatibility harness uses the approved mock entrypoint

- **WHEN** the repo runs an ACP compatibility or mock validation harness
- **THEN** it SHALL use the approved `agent-acp` mock entrypoint or documented
  replacement rather than `brain-acp-mock`
