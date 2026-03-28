- [x] Add OpenSpec change files for the new `agent-cli` capability and
      bootstrap ownership move
- [x] Add `agent-store` bootstrap loading for `.agents/config.toml` and
      `.agents/AGENTS.md`
- [x] Update `agent-core` project resolution to use the store-owned bootstrap
      loader
- [x] Add the `agent-cli` workspace crate and wire it into the workspace
      manifest
- [x] Implement interactive chat, sessions, and credential management through
      `AgentCore`
- [x] Add or move tests covering bootstrap resolution behavior
- [x] Validate the change with `openspec validate add-agent-cli --strict`
- [x] Run `cargo test --workspace`
