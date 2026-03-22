# atif Specification

## Purpose
TBD - created by archiving change add-harbor-brain-direct-agent. Update Purpose after archive.
## Requirements
### Requirement: Reusable Rust ATIF Crate

The workspace SHALL provide an `atif` crate as the reusable Rust schema and
validation surface for Harbor-compatible ATIF trajectories.

The crate SHALL:

- model Harbor ATIF `v1.6`
- expose reusable Rust types for trajectories, steps, agents, metrics, and
  related content
- validate key structural ATIF constraints

#### Scenario: Rust code imports ATIF schema types

- **WHEN** a workspace crate depends on `atif`
- **THEN** it SHALL be able to construct and serialize Harbor-compatible ATIF
  trajectories using Rust types from that crate

### Requirement: ATIF Validation Enforces Published Shape

The `atif` crate SHALL validate the structural rules relied on by the direct
`brain` export path.

That validation SHALL include:

- sequential `step_id` ordering
- valid tool-call references from observations
- schema-version-aware field support

#### Scenario: Invalid ATIF is rejected before export

- **WHEN** a caller constructs an ATIF trajectory with invalid step ordering or
  invalid tool-call references
- **THEN** ATIF validation SHALL fail

### Requirement: Harbor Compatibility Is Cross-Checked

The repo SHALL cross-check the Rust `atif` crate against Harbor's reference
models in test coverage.

#### Scenario: Harbor accepts a Rust-emitted valid trajectory

- **WHEN** the Harbor compatibility integration test runs on a valid ATIF
  fixture emitted from the Rust crate
- **THEN** Harbor's Pydantic models SHALL accept it

#### Scenario: Harbor rejects an invalid Rust-emitted trajectory

- **WHEN** the Harbor compatibility integration test runs on an invalid ATIF
  fixture emitted from the Rust crate
- **THEN** Harbor's Pydantic models SHALL reject it

