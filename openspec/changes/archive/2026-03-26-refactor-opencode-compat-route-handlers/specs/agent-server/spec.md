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

#### Scenario: Route-family file owns its HTTP handlers
- **WHEN** a compat route family such as session, provider, project, files, or
  TUI is implemented
- **THEN** its HTTP-facing Axum handlers SHALL live in the corresponding
  `routes/*.rs` module
- **AND** shared non-HTTP helpers MAY remain outside the route-family file

