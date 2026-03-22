## MODIFIED Requirements

### Requirement: Harbor Benchmark Runs Use One Dedicated Runner Interface
The repo SHALL expose one dedicated Harbor benchmark runner script that
requires an agent name, a Harbor dataset id, and an optional Harbor task name.

The runner script command shape SHALL be:

- `./scripts/harbor-run.sh <agent> <dataset> [task-name]`

The repo MAY also expose that same runner through `just`.

When a contributor provides a dataset without a task name, the runner SHALL
default to the full filtered dataset unless `HARBOR_N_TASKS` is explicitly set.

When a contributor provides a dataset and a task name, the runner MAY keep a
bounded single-task default.

#### Scenario: Contributor runs a benchmark through the repo interface
- **WHEN** a contributor invokes `./scripts/harbor-run.sh codex terminal-bench-sample@2.0 regex-log`
- **THEN** the repo SHALL dispatch the built-in Codex Harbor run for the
  requested dataset and task
- **AND** the same command shape SHALL be available for the other supported
  reference agent surfaces
- **AND** the repo MAY provide `just harbor-run codex terminal-bench-sample@2.0 regex-log` as a thin wrapper

#### Scenario: Dataset-only run defaults to the full dataset
- **WHEN** a contributor invokes `./scripts/harbor-run.sh brain-acp terminal-bench@2.0`
- **AND** `HARBOR_N_TASKS` is not set
- **THEN** the runner SHALL not force a one-task bound
- **AND** Harbor SHALL be allowed to run the full dataset
