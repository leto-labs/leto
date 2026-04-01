# repo-tooling Specification Delta

## ADDED Requirements

### Requirement: Workspace Layout Must Group Standalone Provider And Tool Crates By Ecosystem

The workspace MUST group standalone provider crates under
`crates/agent-provider/` and standalone tool crates under `crates/agent-tool/`.

This grouping SHALL preserve existing Cargo package names and Rust import
surfaces while changing the filesystem layout.

#### Scenario: Contributor inspects standalone crate ecosystems
- **WHEN** a contributor inspects the `crates/` directory
- **THEN** standalone provider crates appear under `crates/agent-provider/`
- **AND** standalone tool crates appear under `crates/agent-tool/`
- **AND** the workspace still resolves the same package names through Cargo
