# agent-tool-files Specification Delta

## ADDED Requirements

### Requirement: Agent Tool Files Must Provide Canonical File And Workspace Tools

The system MUST provide an `agent-tool-files` crate that owns the first-party
file, workspace, and search-oriented tool implementations.

This SHALL include the canonical implementations for:

- `file_read`
- `file_write`
- `file_edit`
- `apply_patch`
- `list_directory`
- `glob_search`
- `grep`

#### Scenario: Native file tool family registers its canonical tools
- **WHEN** a caller builds or extends the native file tool family registry
- **THEN** the resulting tool definitions include the canonical file and
  workspace tool names
- **AND** those tools execute through typed request/response contracts and
  swappable drivers

### Requirement: Agent Tool Files Must Generate Schemas From Typed Contracts

The system MUST generate tool input and output schemas directly from the typed
request/response types owned by `agent-tool-files`.

The implementation MUST use direct `schemars` output unless a concrete
compatibility problem requires a normalization layer.

#### Scenario: File tool schemas preserve typed contract details
- **WHEN** an `agent-tool-files` request or response type derives the schema traits
- **THEN** its generated tool definition preserves required fields
- **AND** field or object descriptions may come from Rust doc comments or
  explicit schema attributes
