# brain-benchmarks Specification

## Purpose
Operational benchmark workflow for evaluating agent harnesses against
Harbor-managed tasks, including Harbor built-in agents and repo-local ACP
agents.
## Requirements
### Requirement: Harbor Is Installed Externally
The repo SHALL document Harbor as an externally installed CLI tool rather than
as a submodule or runtime dependency of this workspace.

The documented installation path SHALL pin a known Harbor version.

#### Scenario: Contributor installs Harbor
- **WHEN** a contributor follows the benchmark setup guide
- **THEN** they SHALL be instructed to install Harbor with `uv tool install`
- **AND** the command SHALL pin a specific Harbor version

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

### Requirement: The Repo Defines A Common Harbor Benchmark Ladder
The repo SHALL define a common Harbor benchmark ladder for bounded evaluation:

- `hello-world@1.0`
- `terminal-bench-sample@2.0/regex-log`
- `terminal-bench-sample@2.0/chess-best-move`
- `terminal-bench-sample@2.0/sqlite-with-gcov`

This ladder SHALL be documented as the repo's common checkpoint progression.

#### Scenario: Contributor chooses a benchmark target
- **WHEN** a contributor wants to run a standard Harbor benchmark checkpoint
- **THEN** they SHALL be able to choose one of the documented benchmark targets
- **AND** each target SHALL map to a concrete Harbor dataset/task combination

### Requirement: Harbor Benchmark Runs Use One Dedicated Runner Interface

The repo SHALL expose one dedicated Harbor benchmark runner script that
requires an agent name, a Harbor dataset id, and an optional Harbor task name.

The runner script command shape SHALL be:

- `./scripts/harbor-run.sh <agent> <dataset> [task-name]`

The repo MAY also expose that same runner through `just`.

The runner SHALL accept Harbor environment selection and core Harbor boolean
environment controls through env vars aligned to Harbor's CLI:

- `HARBOR_ENV` SHALL map to Harbor's `--env`
- `HARBOR_FORCE_BUILD` SHALL map to `--force-build` / `--no-force-build`
- `HARBOR_DELETE` SHALL map to `--delete` / `--no-delete`

The default runner behavior SHALL remain Docker-backed with forced builds and
delete-on-completion unless those env vars override it.

#### Scenario: Contributor runs the direct brain benchmark surface

- **WHEN** a contributor invokes `./scripts/harbor-run.sh brain hello-world@1.0`
- **THEN** Harbor SHALL load the repo-local direct `brain` agent through
  `--agent-import-path`
- **AND** that agent SHALL launch the local `brain` binary directly inside the
  Harbor task container

#### Scenario: Contributor switches the common runner to Daytona

- **WHEN** a contributor sets `HARBOR_ENV=daytona` and runs the common Harbor
  runner
- **THEN** the repo SHALL invoke Harbor with `--env daytona`
- **AND** the runner SHALL NOT require local Docker as a prerequisite for that
  invocation
- **AND** the dataset/task interface SHALL remain unchanged

### Requirement: Built-In Harbor Agents Run Through Harbor's Native Agent Path
The built-in reference surfaces SHALL use Harbor's built-in agent path.

The repo SHALL support at least:

- `codex`
- `mini-swe-agent`
- `terminus-2`

#### Scenario: Contributor runs a built-in benchmark surface
- **WHEN** a contributor runs `just harbor-run mini-swe-agent terminal-bench-sample@2.0 regex-log`
- **THEN** the repo SHALL invoke Harbor with `--agent mini-swe-agent`
- **AND** the run SHALL use the selected dataset/task inputs

### Requirement: Repo-Local ACP Agents Run Through Harbor Import Paths
The repo SHALL support repo-local ACP benchmark agents loaded through
`--agent-import-path`.

The supported repo-local ACP benchmark agents SHALL be:

- `tools.harbor.agents.acp_codex:AcpCodexAgent`
- `tools.harbor.agents.acp_brain:AcpBrainAgent`

#### Scenario: Contributor runs an ACP benchmark surface
- **WHEN** a contributor runs `just harbor-run codex-acp terminal-bench-sample@2.0 regex-log`
- **THEN** Harbor SHALL load the repo-local ACP Codex agent through
  `--agent-import-path`
- **AND** that agent SHALL launch its backend inside Harbor's Docker environment

### Requirement: Repo-Local ACP Codex Uses The Shared ACP Base
The supported Harbor ACP Codex path SHALL use the shared Harbor ACP client and
shared ACP base, with a Codex-specific backend implementation.

#### Scenario: Contributor reads the Codex ACP Harbor workflow
- **WHEN** a contributor reads the Harbor benchmark docs or benchmark spec
- **THEN** they SHALL see `codex-acp` described as a repo-local ACP benchmark
  surface built on the shared Harbor ACP client and base
- **AND** they SHALL see it documented as a bounded real-task validation path

### Requirement: Repo-Local ACP Brain Uses The Shared ACP Base
The supported Harbor ACP Brain path SHALL use the shared Harbor ACP client and
shared ACP base, with a Brain-specific backend implementation.

#### Scenario: Contributor reads the Brain ACP Harbor workflow
- **WHEN** a contributor reads the Harbor benchmark docs or benchmark spec
- **THEN** they SHALL see `brain-acp` described as a repo-local ACP benchmark
  surface built on the shared Harbor ACP client and base
- **AND** they SHALL see it documented as validated on `hello-world@1.0` and
  bounded real-task probes rather than the first stable comparison baseline

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

