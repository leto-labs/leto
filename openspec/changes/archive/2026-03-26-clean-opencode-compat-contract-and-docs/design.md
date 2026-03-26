# Design: Clean OpenCode compat contract and documentation architecture

## Decisions

### 1. Organize compat DTOs by domain

OpenCode DTOs will live in domain modules that match the compat API surface:

- `files`
- `project`
- `provider`
- `session`
- `permission`
- `question`
- `global`
- `events`
- `mcp`
- `experimental`
- `pty`
- `tui`
- `errors`
- `common`

`types/mod.rs` remains the public compat type surface and re-exports those
modules.

### 2. Preserve one runtime/doc contract

Route handlers and generated OpenAPI will continue to use the same DTOs.
This change must not reintroduce a docs-only DTO layer.

### 3. Keep only justified doc helpers

OpenAPI helper logic is only allowed when:

- it adapts generator output to OpenAPI format conventions
- it documents a transport shape that cannot be inferred cleanly
- it preserves route-parity verification without reintroducing legacy runtime
  doc assembly

Helpers that only compensate for poor DTO naming or structure must be removed by
fixing the DTOs instead.

Legacy compat doc assembly modules are removed entirely; remaining helper logic
must stay small, local, and justified.

## Non-goals

- No change to the external compat wire contract
- No change to the canonical `/v1` API
