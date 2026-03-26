# Tasks: eliminate-opencode-doc-only-dtos

- [x] Replace the compat `types::docs` module with a shared contract module
- [x] Remove remaining compat handler signatures that used
      `Query<BTreeMap<String, String>>`
- [x] Use shared contract DTOs for compat config, provider, file, project,
      session, PTY, workspace, worktree, permission, question, and MCP runtime
      payloads where the contract is stable
- [x] Remove the docs-only `ConfigDocument` shim and use the OpenCode config DTO
      directly in config handlers
- [x] Keep the prefix-aware `/doc` parity test green
- [x] Run `cargo test -p agent-server`
- [x] Run `cargo test -p agent-core-remote -p agent-server`
- [x] Validate the OpenSpec change with
      `openspec validate eliminate-opencode-doc-only-dtos --strict`
