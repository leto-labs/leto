# brain-tools Specification Delta

## ADDED Requirements

### Requirement: ApplyPatch Tool

The system SHALL provide an `apply_patch` tool for multi-file unified diff
application.

The tool SHALL:

- accept a `patch` string parameter
- support file creation, modification, and deletion
- fail clearly when a patch cannot be parsed or applied

#### Scenario: Patch updates an existing file

- **WHEN** `apply_patch` receives a valid unified diff for an existing file
- **THEN** it SHALL update that file
- **AND** return a confirmation result

#### Scenario: Patch creates or deletes files

- **WHEN** `apply_patch` receives a unified diff using `/dev/null`
- **THEN** it SHALL create or delete the corresponding file as appropriate

### Requirement: ListDirectory Tool

The system SHALL provide a `list_directory` tool for repository structure
inspection.

The tool SHALL:

- accept `path` and optional `depth`
- return deterministic tree-like output
- ignore common generated directories such as `.git`, `node_modules`, and
  `target`

#### Scenario: Directory listing returns tree output

- **WHEN** `list_directory` is called on a nested directory
- **THEN** it SHALL return a readable tree-like listing
- **AND** omit ignored generated directories

### Requirement: Grep Prefers Ripgrep When Available

The `grep` tool SHALL keep its public name and schema while allowing a
ripgrep-backed implementation internally.

The native preset SHALL:

- use a ripgrep-backed driver when `rg` is available
- fall back to the existing native grep implementation otherwise

#### Scenario: Native tool preset keeps grep contract stable

- **WHEN** `native_tools()` is constructed
- **THEN** it SHALL still expose a tool named `grep`
- **AND** the choice of native or ripgrep-backed implementation SHALL remain an
  internal detail
