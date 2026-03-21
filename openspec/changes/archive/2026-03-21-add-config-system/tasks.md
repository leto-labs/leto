# Tasks: add-config-system

## Completed (Store Decomposition + Project Entity)

- [x] Define `ProjectConfig` struct wrapping `AgentConfig` with `#[serde(flatten)]`
- [x] Define `Project` struct with `id`, `name`, `root`, `config`, timestamps
- [x] Define `ProjectUpdate` struct
- [x] Decompose `Store` trait into `ProjectStore`, `SessionStore`, `MessageStore` sub-traits
- [x] Add composite `Store` supertrait with blanket impl
- [x] Add `project_id` field to `Session` struct
- [x] Update `InMemoryStore` to implement all three sub-traits
- [x] Update `FileStore` with project-scoped directory layout
- [x] Update `Brain` to hold `Project`, derive `AgentConfig` from `project.config`
- [x] Update examples (`cli-echo`, `cli-local`) for new `Brain::new()` signature
- [x] Verify clean build (`cargo check`)

## Remaining (Config Resolution)

- [x] Create `brain-config` crate or module
- [x] Implement `resolve_fs_config(root: &Path) -> Result<ProjectConfig>` utility
- [x] Implement TOML config parsing with `toml` crate
- [x] Implement config discovery: walk from cwd upward looking for `.agents/config.toml`
- [x] Implement env var interpolation in config values (`$ENV_VAR` syntax)
- [x] Implement layered config resolution (defaults → global `~/.brain/config.toml` → project `.agents/config.toml`)
- [x] Implement AGENTS.md runtime discovery (root-level as system prompt)
- [x] Implement nested AGENTS.md discovery (per-directory context injection)
- [x] Write unit tests for config parsing, discovery, and layering

## Validation Notes

- Project bootstrap coverage for `.agents/config.toml` plus root `.agents/AGENTS.md`
  exists in `crates/brain-core/src/brain.rs` tests.
- The shipped config scope is intentionally limited to layered agent config and
  AGENTS discovery. Strict unknown-key validation and richer provider/tool/MCP
  schema are follow-on work, not blockers for archiving this change.
