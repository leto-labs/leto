## ADDED Requirements

### Requirement: Legacy Brain Source Is Archived Locally

The repository SHALL keep legacy `brain-*` source only in a local gitignored
archive tree rooted at `archive/brain/`.

The committed workspace SHALL NOT track `crates/brain-*`.

#### Scenario: Contributor keeps a local legacy reference tree

- **WHEN** a contributor needs to inspect legacy `brain-*` source during
  migration cleanup
- **THEN** they SHALL find it under `archive/brain/`
- **AND** Git SHALL ignore that local archive tree

#### Scenario: Committed workspace excludes legacy crates

- **WHEN** a contributor inspects the committed workspace manifests
- **THEN** `crates/brain-*` SHALL not appear as tracked workspace members or
  tracked workspace dependencies

### Requirement: Live Repo Tooling Excludes Archived Brain Surfaces

The committed repo tooling SHALL NOT depend on archived `brain-*` crates or
their direct Harbor surfaces.

Legacy-only examples, scripts, and Harbor helpers MAY remain only inside the
local `archive/brain/` tree.

#### Scenario: Harbor runner rejects archived Brain agents

- **WHEN** a contributor invokes the committed Harbor runner with a legacy
  archived Brain agent
- **THEN** the runner SHALL fail with an explicit archive message
- **AND** it SHALL NOT try to build or launch archived `brain-*` crates

#### Scenario: Legacy-only helpers are no longer tracked live tooling

- **WHEN** a contributor inspects the committed examples, scripts, and Harbor
  helpers
- **THEN** legacy-only Brain helpers SHALL not remain in those live committed
  paths
