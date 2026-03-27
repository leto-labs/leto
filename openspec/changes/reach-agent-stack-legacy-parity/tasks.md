- [x] Split the completed provider-credential tranche into a focused OpenSpec
      change so it can be archived independently.
- [ ] Extend `agent-store` requirements to persist parity-required project
      defaults.
- [ ] Extend `agent-core` requirements to cover project bootstrap, prompt/loop
      defaults, and remaining assembly behavior.
- [ ] Add `agent-loops` parity requirements for strategies equivalent to legacy
      `robust`, `terminus2`, and `terminus-kira`.
- [ ] Extend `agent-tools` requirements to cover terminal-session parity.
- [ ] Extend `agent-server` requirements so documented compat auth and PTY
      routes require real implementations rather than placeholders.
- [ ] Implement project bootstrap and persisted-default parity in `agent-core`
      and `agent-store`.
- [ ] Implement advanced loop parity in `agent-loops`.
- [ ] Implement terminal-session parity in `agent-tools` and wire it through
      the default core assembly.
- [ ] Replace placeholder compat auth and PTY behavior in `agent-server` with
      real core/runtime-backed implementations.
- [x] Validate the change with `openspec validate
      reach-agent-stack-legacy-parity --strict`.
- [ ] Run `cargo test --workspace` once implementation is complete.
