## MODIFIED Requirements
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
