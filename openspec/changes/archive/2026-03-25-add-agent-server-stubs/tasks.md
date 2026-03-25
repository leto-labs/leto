# Tasks: add-agent-server-stubs

## Implementation

- [x] Split `agent-core` into a shared `AgentCore` trait and a concrete
      `AgentCoreNative` implementation
- [x] Keep store access on the shared `AgentCore` surface for future proxy-store
      parity
- [x] Add a new `agent-server` workspace crate hosted around `Arc<dyn AgentCore>`
- [x] Expose a canonical `/v1` HTTP surface with health/status, project/session
      access, NDJSON turn streaming, and SSE runtime-bus events
- [x] Expose an OpenCode-compatible secondary surface under
      `/v1/compat/opencode`
- [x] Make low-risk compatibility reads real and return explicit
      `501 not_implemented` responses for unresolved compat flows
- [x] Add tests covering trait-object use, canonical HTTP routes, and compat
      stub behavior
- [x] Validate the change with `openspec validate add-agent-server-stubs --strict`
