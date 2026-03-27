## ADDED Requirements

### Requirement: Standalone ATIF Workspace Uses A Pinned SSH Submodule

The repository SHALL vendor the standalone public `atif-rust` repository as a
Git submodule under `submodules/atif-rust`.

That submodule SHALL use the SSH remote
`git@github.com:leto-labs/atif-rust.git` and SHALL stay pinned to an explicit
commit in Git history.

#### Scenario: Contributor initializes repository submodules

- **WHEN** a contributor runs `git submodule update --init --recursive`
- **THEN** the standalone ATIF repository SHALL appear under
  `submodules/atif-rust`
- **AND** the submodule SHALL resolve from the configured SSH remote rather than
  an HTTPS URL

### Requirement: Vendored ATIF Submodule Is Bootstrap-Ready

The vendored `submodules/atif-rust` repository SHALL be usable as a standalone
Rust workspace for the public `atif` crate.

That standalone workspace SHALL include:

- a workspace root manifest
- the seeded `atif` crate source and tests
- repository documentation and licensing suitable for public use
- reproducible Harbor compatibility coverage that does not depend on this
  monorepo's local repocache layout

#### Scenario: Contributor enters the vendored standalone repo

- **WHEN** a contributor opens `submodules/atif-rust`
- **THEN** they SHALL find a runnable Rust workspace for the public `atif` crate
- **AND** the Harbor compatibility flow SHALL use a public Harbor dependency
  path rather than a path into this monorepo
