## MODIFIED Requirements

### Requirement: The Repo Defines Five Harbor Reference Agent Surfaces

The repo SHALL define six Harbor reference agent surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local direct `brain`
- repo-local `codex-acp`
- repo-local `brain-acp`

The repo SHALL document the current support level of each surface.

#### Scenario: Contributor reads the benchmark workflow

- **WHEN** a contributor reads the Harbor benchmark docs or benchmark spec
- **THEN** they SHALL see all six reference agent surfaces listed
- **AND** they SHALL see built-in agents distinguished from repo-local direct
  and ACP agents
- **AND** they SHALL see which surfaces are stable baselines versus bounded
  validation paths

### Requirement: Harbor Benchmark Runs Use One Dedicated Runner Interface

The repo SHALL expose one dedicated Harbor benchmark runner script that
requires an agent name, a Harbor dataset id, and an optional Harbor task name.

The runner script command shape SHALL be:

- `./scripts/harbor-run.sh <agent> <dataset> [task-name]`

The repo MAY also expose that same runner through `just`.

#### Scenario: Contributor runs the direct brain benchmark surface

- **WHEN** a contributor invokes `./scripts/harbor-run.sh brain hello-world@1.0`
- **THEN** Harbor SHALL load the repo-local direct `brain` agent through
  `--agent-import-path`
- **AND** that agent SHALL launch the local `brain` binary directly inside the
  Harbor task container

## ADDED Requirements

### Requirement: Repo-Local Direct Brain Runs Through Harbor Import Path

The repo SHALL support a repo-local direct Harbor `brain` agent loaded through
`--agent-import-path`.

This surface SHALL:

- run the local `brain` binary directly
- avoid ACP entirely
- use explicit API-key auth passed through the Harbor runner surface

#### Scenario: Direct brain Harbor run avoids host brain state

- **WHEN** Harbor runs the repo-local direct `brain` surface
- **THEN** the run SHALL NOT require host `~/.brain/config.toml`
- **AND** it SHALL NOT require host `~/.brain/credentials`
- **AND** the selected API key SHALL be provided explicitly through the Harbor
  runner path
