## MODIFIED Requirements

### Requirement: The Repo Defines Five Harbor Reference Agent Surfaces

The repo SHALL define six Harbor reference agent surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local direct `agent`
- repo-local `codex-acp`
- repo-local `agent-acp`

The repo SHALL document the current support level of each surface.

#### Scenario: Contributor reads the benchmark workflow

- **WHEN** a contributor reads the Harbor benchmark docs or benchmark spec
- **THEN** they SHALL see all six reference agent surfaces listed
- **AND** they SHALL see built-in agents distinguished from repo-local direct
  and ACP agents
- **AND** they SHALL see which surfaces are stable baselines versus bounded
  validation paths

### Requirement: Repo-Local ACP Agents Run Through Harbor Import Paths

The repo SHALL support repo-local ACP benchmark agents loaded through
`--agent-import-path`.

The supported repo-local ACP benchmark agents SHALL be:

- `tools.harbor.agents.acp_codex:AcpCodexAgent`
- `tools.harbor.agents.agent_acp:HarborAcpAgent`

#### Scenario: Contributor runs an ACP benchmark surface

- **WHEN** a contributor runs `just harbor-run codex-acp terminal-bench-sample@2.0 regex-log`
- **THEN** Harbor SHALL load the repo-local ACP Codex agent through
  `--agent-import-path`
- **AND** that agent SHALL launch its backend inside Harbor's Docker environment

### Requirement: Repo-Local ACP Agent Uses The Shared ACP Base

The supported Harbor ACP agent path SHALL use the shared Harbor ACP client and
shared ACP base, with an agent-specific backend implementation.

#### Scenario: Contributor reads the agent ACP Harbor workflow

- **WHEN** a contributor reads the Harbor benchmark docs or benchmark spec
- **THEN** they SHALL see `agent-acp` described as a repo-local ACP benchmark
  surface built on the shared Harbor ACP client and base
- **AND** they SHALL see it documented as a loop-validation surface on the live
  `agent-*` stack rather than as an archived legacy path

### Requirement: Repo-Local Direct Agent Runs Through Harbor Import Path

The repo SHALL support a repo-local direct Harbor `agent` agent loaded through
`--agent-import-path`.

This surface SHALL:

- run the local `agent` binary directly
- avoid ACP entirely
- use explicit API-key auth passed through the Harbor runner surface

#### Scenario: Direct agent Harbor run avoids host agent state

- **WHEN** Harbor runs the repo-local direct `agent` surface
- **THEN** the run SHALL NOT require host `~/.agent/config.toml`
- **AND** it SHALL NOT require host `~/.agent/credentials`
- **AND** the selected API key SHALL be provided explicitly through the Harbor
  runner path
