- [x] Extend `agent-store` project config to persist prompt and default loop
      bootstrap defaults.
- [x] Add project-local bootstrap resolution in `agent-core` for
      `.agents/config.toml`.
- [x] Apply root `.agents/AGENTS.md` as a fallback only when no explicit prompt
      exists.
- [x] Use persisted project defaults during loop/runtime resolution.
- [x] Seed the project prompt into the first turn transcript once and preserve
      historical prompt state across later config edits.
- [x] Add store/core tests covering config loading, prompt fallback, default
      loop resolution, and transcript preservation.
- [x] Validate with `cargo test --workspace`.
- [x] Validate with `openspec validate
      add-agent-core-project-bootstrap-defaults --strict`.
