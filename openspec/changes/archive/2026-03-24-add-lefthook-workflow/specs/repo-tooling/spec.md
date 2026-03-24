## ADDED Requirements

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
