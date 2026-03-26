# agent-server Delta Spec

## MODIFIED Requirements

### Requirement: OpenCode Compatibility Surface Is Secondary
The system MUST expose an OpenCode-compatible secondary surface under
`/v1/compat/opencode`.

That compatibility surface MUST remain separate from the canonical API and MUST
target OpenCode release `v1.3.2` as its compatibility baseline.

The compat implementation SHALL organize HTTP-facing route handlers by
route-family module under `compat/opencode/routes/` rather than keeping them in
a monolithic compat handler file.

The compat implementation SHALL use shared OpenCode contract DTOs for both
runtime request/response handling and generated OpenAPI, rather than keeping a
separate docs-only DTO layer.

#### Scenario: Query DTO used by both runtime and OpenAPI
- **WHEN** a compat route documents a stable query contract such as directory,
  workspace, file lookup, or experimental session listing parameters
- **THEN** the route handler SHALL parse that same DTO at runtime
- **AND** the generated `/v1/compat/opencode/doc` output SHALL describe that same
  DTO without a parallel schema-only shim

#### Scenario: Compat response adapter uses contract DTOs
- **WHEN** compat runtime state or helpers expose OpenCode payloads for projects,
  sessions, messages, PTYs, providers, files, workspaces, questions,
  permissions, or MCP status
- **THEN** those payloads SHALL be constructed through shared contract DTOs
- **AND** exact prefix-aware `/doc` parity SHALL remain green
