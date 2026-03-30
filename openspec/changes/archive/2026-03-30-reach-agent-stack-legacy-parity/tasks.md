- [x] Split the completed provider-credential tranche into a focused OpenSpec
      change so it can be archived independently.
- [x] Split the completed bootstrap/defaults tranche into a focused OpenSpec
      change so it can be archived independently.
- [x] Add `agent-loops` parity requirements for strategies equivalent to legacy
      `robust`, `terminus2`, and `terminus-kira`.
- [x] Extend the parity requirements to cover runtime-native terminal-session
      behavior on the PTY/session surface.
- [x] Remove the stale `agent-server` compatibility requirements from this
      change because legacy `brain-server` never shipped that surface.
- [x] Implement advanced loop parity in `agent-loops`.
- [x] Implement terminal-session parity through runtime-native PTY/session
      orchestration and wire the advanced loops through the default core
      assembly.
- [x] Validate the change with `openspec validate
      reach-agent-stack-legacy-parity --strict`.
- [x] Run `cargo test --workspace` once implementation is complete.
- [x] Archive the cleaned-up loop/runtime parity tranche so the remaining
      legacy migration follow-up can move to ACP-specific tracking.
