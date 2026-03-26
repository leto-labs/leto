# Design: eliminate-opencode-doc-only-dtos

## Decision

OpenCode compat DTOs are treated as the real wire contract, not as schema-only
stand-ins. The same Rust types should be used:

- by Axum extractors and JSON responses
- by compat state where the state already mirrors external payloads
- by `aide`/`schemars` when generating `/v1/compat/opencode/doc`

## Consequences

- route handlers that previously accepted loose query maps now accept typed query
  DTOs
- compat adapters such as project/session/message/PTy/provider/file/MCP
  conversion move from `Value` construction toward contract DTO construction
- helper modules can still normalize generated OpenAPI for parity, but they no
  longer rely on a separate docs-only DTO layer

## Non-Goals

- no change to the exact OpenCode parity target
- no change to the canonical `/v1` API
- no attempt to eliminate legitimate free-form JSON fields where the OpenCode
  contract itself is open-ended
