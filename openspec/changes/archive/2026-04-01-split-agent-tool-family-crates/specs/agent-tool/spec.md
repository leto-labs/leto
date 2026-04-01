# agent-tool Specification Delta

## ADDED Requirements

### Requirement: Agent Tool Must Provide The Shared Tool SDK

The system MUST provide an `agent-tool` crate that owns the shared tool
executor protocol used by runtimes and first-party tool families.

The crate MUST define the shared tool call/result contracts, tool approval
types, tool-scoped errors, and the standard erased tool executor trait.

#### Scenario: Runtime depends on the shared tool SDK
- **WHEN** a runtime crate needs to execute or approve tool calls
- **THEN** it uses the shared types and traits from `agent-tool`
- **AND** it SHALL NOT need to define those contracts inside `agent-runtime`

### Requirement: Agent Tool Must Provide A Standard Registry Path

The system MUST provide a reusable registry path in `agent-tool` so callers can
compose native or future extension-backed tool implementations through one
standard executor surface.

#### Scenario: Caller composes multiple tool families
- **WHEN** a caller registers `agent-tool`, `agent-tool-files`, and
  `agent-tool-process` tool implementations into one registry
- **THEN** the registry exposes one provider-visible tool definition set
- **AND** dispatches tool calls by name through the shared executor interface
