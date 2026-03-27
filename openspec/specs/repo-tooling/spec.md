# repo-tooling Specification

## Purpose
Repository-local developer tooling conventions, including committed Git hook
behavior used to enforce staged-file formatting workflows.
## Requirements
### Requirement: Repository Uses Committed Lefthook Configuration

The repository SHALL keep its local Git hook configuration in committed
`lefthook` configuration rather than relying on ad hoc untracked hooks.

#### Scenario: Contributor installs repository hooks

- **WHEN** a contributor has `lefthook` available locally
- **THEN** running `lefthook install` SHALL install the repository's configured
  hooks

### Requirement: Pre-Commit Hooks Format Staged Rust Files

The repository SHALL format staged Rust files before commit without broad
workspace formatting.

#### Scenario: Only staged Rust files are formatted

- **WHEN** the `pre-commit` hook runs
- **THEN** it SHALL run `rustfmt` over the staged Rust files
- **AND** it SHALL limit formatting to the Rust files already staged for commit
- **AND** it SHALL NOT run `cargo fmt --all`

#### Scenario: Hook-restaged formatting is preserved

- **WHEN** the `pre-commit` hook formats a staged Rust file
- **THEN** the formatted version SHALL remain staged for the commit

### Requirement: OpenCode UI Validation Uses A Pinned Submodule
The repository SHALL keep the runnable OpenCode UI validation consumer as a Git
submodule under `submodules/opencode`.

That submodule SHALL use the synced `leto-labs/opencode` fork as its remote,
but it SHALL be pinned to the exact commit corresponding to the compatibility
release currently used by the repository's OpenCode reference material rather
than to a floating branch tip.

#### Scenario: Contributor checks out the OpenCode UI validation target
- **WHEN** a contributor initializes repository submodules
- **THEN** the OpenCode validation consumer SHALL appear under
  `submodules/opencode`
- **AND** it SHALL resolve to the exact pinned compatibility commit rather than
  to a moving `dev` branch

#### Scenario: Compatibility pin is updated later
- **WHEN** the repository intentionally updates its OpenCode compatibility
  target
- **THEN** the submodule pin SHALL move together with that approved target
- **AND** the change SHALL remain explicit in Git history

### Requirement: OpenCode UI Validation Must Stay Release-Aligned
The repository SHALL keep the OpenCode UI validation submodule aligned with the
same approved OpenCode release used by the compatibility contract artifacts and
implemented compat work until a deliberate compatibility upgrade is approved.

#### Scenario: Contributor proposes following upstream dev
- **WHEN** a contributor proposes moving the validation submodule to a newer
  fork or upstream branch tip without also updating the repository's approved
  compatibility target
- **THEN** that proposal SHALL be rejected
- **AND** the submodule SHALL remain pinned to the release-aligned commit

#### Scenario: Repository reference pins drift apart
- **WHEN** repository-local OpenCode reference pins disagree, such as the
  submodule pin, local repocache manifest, or approved compat target
- **THEN** contributors SHALL treat that as drift to resolve
- **AND** SHALL NOT introduce a new mismatched pin for the validation submodule

### Requirement: OpenCode UI Edits Must Stay Minimal
Local changes to the validation submodule SHALL be allowed only for hard
blockers discovered during compatibility testing, such as auth transport,
CORS, or platform-specific networking seams.

Those local changes SHALL NOT be used as a substitute for missing server-side
compatibility behavior.

#### Scenario: Contributor proposes a frontend workaround for a missing compat route
- **WHEN** a frontend patch is proposed mainly because `agent-server` is
  missing required compatibility behavior
- **THEN** that patch SHALL be treated as invalid
- **AND** the required fix SHALL remain on the server-side compatibility layer

#### Scenario: Contributor patches the UI for a real transport seam
- **WHEN** a narrowly scoped UI patch is needed to address a true auth, CORS,
  or platform transport blocker
- **THEN** that patch MAY be accepted
- **AND** the change SHALL stay explicit, minimal, and documented

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

