## ADDED Requirements

### Requirement: Agent Tools Must Provide Typed V2 Tool Implementations

The system MUST provide an `agent-tools` crate that owns the concrete v2 tool
surface for local runtime assembly.

The crate MUST keep typed Rust request/response contracts and swappable driver
traits while exposing an erased runtime-facing `ToolExecutor` surface.

#### Scenario: Native tool registry backs the runtime executor
- **WHEN** a caller builds the default native `agent-tools` registry
- **THEN** it exposes provider-visible tool definitions
- **AND** dispatches tool calls by name through typed request/response handlers

### Requirement: Agent Tools Must Generate Schemas Directly From Typed Contracts

The system MUST generate tool input and output schemas directly from the typed
request/response types used by `agent-tools`.

The v1 implementation MUST use direct `schemars` output and MUST NOT add a
schema-normalization layer unless a concrete compatibility problem requires it.

#### Scenario: Tool schemas preserve required fields and descriptions
- **WHEN** an `agent-tools` request/response type derives the schema traits
- **THEN** its generated tool definition preserves required fields
- **AND** object/field descriptions may come from Rust doc comments or explicit schema attributes

