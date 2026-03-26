# Tasks: implement-agent-server-runtime

## Baseline

- [x] Refresh the local OpenCode repocache clone and pin compatibility checks to
      tag `v1.3.2`
- [x] Treat `openapi/opencode.json` as the contract artifact for compatibility
      inventory and schema checks

## Canonical Runtime

- [x] Make all currently exposed canonical `/v1` routes fully real and remove
      placeholder behavior from the canonical API
- [x] Add any missing server-local support needed for canonical status, runtime
      introspection, and stable error handling
- [x] Add canonical integration coverage for turn streaming, cancellation,
      projects, sessions, messages, trajectories, providers, and credentials

## OpenCode Compatibility

- [x] Replace compat stubs with real implementations for the full OpenCode
      `v1.3.2` route surface declared in `openapi/opencode.json`
- [x] Add server-local compat subsystems for config, provider auth, permission,
      question, PTY, MCP, workspaces, worktrees, TUI control, and related
      OpenCode-only surfaces
- [x] Align compat SSE behavior, request parsing, status codes, and CORS with
      OpenCode expectations closely enough for later UI testing
- [x] Implement `/v1/compat/opencode/doc` from the compat implementation rather
      than leaving a static stub
- [x] Refactor the compat implementation into `compat/opencode` route-family
      modules instead of a single monolithic router file
- [x] Introduce a typed DTO layer for the full set of body-bearing OpenCode
      compat operations and wire those DTOs into generated docs
- [x] Remove all runtime use of `openapi/opencode.json` from the compat server
      implementation

## Verification

- [x] Add route inventory tests proving `/v1/compat/opencode` covers every
      path/method in `openapi/opencode.json`
- [x] Add spec parity tests comparing generated compat `/doc` output with
      `openapi/opencode.json` using a prefix-aware comparator
- [x] Add a guardrail test proving runtime compat code does not import
      `openapi/opencode.json`
- [x] Tighten the parity check to exact prefix-aware OpenAPI equality instead of
      partial inventory-only validation
- [x] Validate the OpenSpec change with
      `openspec validate implement-agent-server-runtime --strict`
- [x] Run targeted cargo tests for `agent-server` and the
      `agent-core-remote` + `agent-server` integration slice
