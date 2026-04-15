# Tasks: add-agent-acp-session-modes-and-client-terminal-bridge

## Backend

- [x] Add `agent-acp` requirements for real session modes, available-command
      advertising, and ACP startup store hygiene
- [x] Implement real backend session modes and available commands on
      `agent-acp`
- [x] Implement ACP session models plus `session/set_model` on the real
      backend
- [x] Implement ACP `session/resume`, `session/fork`, and `session/close` on
      the real backend
- [x] Extend real backend session listing beyond cwd-scoped queries
- [x] Preserve ACP `ResourceLink` prompt content and emit session info/config
      updates when backend-owned session state changes
- [x] Implement startup validation and dedicated default agent home behavior
      for `agent-acp` and `agent acp`
- [x] Add or update backend tests for modes, command updates, and startup/home
      resolution

## Nori Fork

- [x] Add `agent-acp-integration` requirements for Nori ACP terminal client
      methods
- [x] Implement ACP terminal client methods in the checked-out Nori fork
- [x] Add or update Nori ACP connection tests for terminal create/output/wait/
      kill/release

## Validation

- [x] Run targeted Rust tests for `agent-acp`, `agent-cli`, and Nori ACP
      connection coverage
- [x] Run ACP smoke validation with `acpx`
- [x] Validate the change with
      `openspec validate add-agent-acp-session-modes-and-client-terminal-bridge --strict`
