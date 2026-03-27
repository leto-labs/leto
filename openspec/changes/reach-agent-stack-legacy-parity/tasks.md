- [x] Split the completed provider-credential tranche into a focused OpenSpec
      change so it can be archived independently.
- [x] Split the completed bootstrap/defaults tranche into a focused OpenSpec
      change so it can be archived independently.
- [x] Add `agent-loops` parity requirements for strategies equivalent to legacy
      `robust`, `terminus2`, and `terminus-kira`.
- [x] Extend the parity requirements to cover runtime-native terminal-session
      behavior on the PTY/session surface.
- [ ] Extend `agent-server` requirements so documented compat auth and PTY
      routes require real implementations rather than placeholders.
- [x] Implement advanced loop parity in `agent-loops`.
- [x] Implement terminal-session parity through runtime-native PTY/session
      orchestration and wire the advanced loops through the default core
      assembly.
- [ ] Replace placeholder compat auth and PTY behavior in `agent-server` with
      real core/runtime-backed implementations.
- [x] Validate the change with `openspec validate
      reach-agent-stack-legacy-parity --strict`.
- [x] Run `cargo test --workspace` once implementation is complete.
