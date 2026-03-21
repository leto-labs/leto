# brain-benchmarks Specification

## Purpose
Operational benchmark workflow for evaluating external agent harnesses against
Harbor-managed tasks before adding a complete `brain-acp` Harbor integration.

## ADDED Requirements

### Requirement: Harbor External Installation
The repo SHALL document Harbor as an externally installed CLI tool rather than
as a submodule or runtime dependency of this workspace.

The documented installation path SHALL pin a known Harbor version.

#### Scenario: Contributor installs Harbor
- **WHEN** a contributor follows the benchmark setup guide
- **THEN** they SHALL be instructed to install Harbor with `uv tool install`
- **AND** the command SHALL pin a specific Harbor version

### Requirement: Bounded Paid Harbor Built-In Baseline
The repo SHALL provide a separate helper for running one paid Harbor benchmark
task with a built-in agent under explicit cost controls.

The helper SHALL default to:

- one task
- one attempt
- one concurrent trial
- an explicit timeout multiplier

#### Scenario: Paid smoke test stays bounded
- **WHEN** a contributor runs the paid Harbor built-in baseline helper
- **THEN** the helper SHALL require the caller to provide an explicit model selection
- **AND** the Harbor invocation SHALL be bounded to a single-task, single-attempt, single-concurrency run by default

### Requirement: Repo-Local Harbor ACP Bridge
The repo SHALL provide a repo-local Harbor custom agent loaded through
`--agent-import-path` that acts as an ACP client and launches an ACP backend
process.

The bridge SHALL be validated first against `codex-acp` on a Harbor registry
dataset task.

#### Scenario: Harbor loads the repo-local ACP bridge
- **WHEN** a contributor runs the Harbor ACP helper
- **THEN** Harbor SHALL load the repo-local custom agent via `--agent-import-path`
- **AND** the custom agent SHALL launch an ACP backend process
- **AND** the ACP bridge SHALL persist artifacts under the Harbor job directory

### Requirement: Repo-Owned Harbor Task Template
The repo SHALL keep a Harbor-compatible local task as a template for future
custom Harbor tasks, but it SHALL NOT be the primary benchmark workflow.

#### Scenario: Contributor needs a custom Harbor task example
- **WHEN** a contributor inspects repo-owned Harbor assets
- **THEN** they SHALL find a local Harbor task template in the repo
- **AND** the main benchmark docs SHALL point primary execution toward Harbor registry datasets

### Requirement: Future `brain-acp` Harbor Integration Is Documented
The repo SHALL document the planned Harbor integration path for `brain-acp`
without requiring that path to be completed in this change.

The documented path SHALL distinguish:

- a generic ACP bridge validated first with `codex-acp`
- a subsequent `brain-acp` backend path
- a richer metrics and trajectory-aware adapter

#### Scenario: Contributor reads Harbor benchmark docs
- **WHEN** a contributor reads the Harbor benchmark documentation
- **THEN** they SHALL find the planned custom-agent path for `brain-acp`
- **AND** the documentation SHALL mark that work as deferred beyond the current baseline implementation
