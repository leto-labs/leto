# Tasks: refactor-opencode-compat-route-handlers

- [x] Move every OpenCode compat HTTP-facing handler from `compat/opencode/mod.rs`
      into the corresponding `routes/*.rs` file
- [x] Leave only shared non-HTTP helpers and compat-wide infrastructure in
      `compat/opencode/mod.rs`
- [x] Keep the prefix-aware `/doc` parity test green throughout the refactor
- [x] Run `cargo test -p agent-server`
- [x] Run `cargo test -p agent-core-remote -p agent-server`
- [x] Validate the OpenSpec change with
      `openspec validate refactor-opencode-compat-route-handlers --strict`
