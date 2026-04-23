## REMOVED Requirements
### Requirement: Workspace Layout Must Group Standalone Provider And Tool Crates By Ecosystem

The workspace no longer keeps standalone provider crates under
`crates/agent-provider/`.

## ADDED Requirements
### Requirement: Shared Crate Families Use Pinned Submodule Workspaces

The repository SHALL vendor the standalone `atif-rust`, `ai-provider-rs`, and
`chat-rs` repositories as pinned Git submodules under `submodules/`.

The root `leto` workspace SHALL consume `atif`, `provider-*`, and `chat-*`
through local `path` dependencies into those submodules rather than tracking
duplicate in-tree crate copies as root workspace members.

#### Scenario: Contributor initializes vendored crate family submodules

- **WHEN** a contributor runs `git submodule update --init --recursive`
- **THEN** `submodules/atif-rust`, `submodules/ai-provider-rs`, and
  `submodules/chat-rs` SHALL be present
- **AND** each one SHALL resolve from its configured pinned submodule commit

#### Scenario: Contributor inspects root Cargo workspace topology

- **WHEN** a contributor inspects the root `Cargo.toml`
- **THEN** `atif`, `provider-*`, and `chat-*` SHALL not appear as root
  `workspace.members`
- **AND** the root workspace SHALL exclude the vendored submodule roots so the
  nested Cargo workspaces remain authoritative
- **AND** the root `[workspace.dependencies]` SHALL point those package names at
  the corresponding `submodules/.../crates/...` paths

### Requirement: Repo Commands Cover Root And Vendored Rust Workspaces

The committed repo commands SHALL make it explicit when verification targets the
root `leto` workspace only versus the full set of vendored Rust workspaces.

#### Scenario: Contributor runs the standard repo test command

- **WHEN** a contributor runs the repo's standard `just test` command
- **THEN** it SHALL run the root `leto` workspace tests
- **AND** it SHALL also run the `atif-rust`, `ai-provider-rs`, and `chat-rs`
  workspace tests

#### Scenario: Contributor reads coverage guidance

- **WHEN** a contributor inspects the committed coverage commands or guidance
- **THEN** the repo SHALL state explicitly if those commands cover only the root
  `leto` workspace rather than the vendored submodule workspaces
