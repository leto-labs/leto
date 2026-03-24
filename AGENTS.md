## Agent Skills Layout

Use a single source of truth for skills so different agents stay aligned:

- `.agents/skills` - canonical install target
- `.claude/skills -> ../.agents/skills` - Claude compatibility symlink
- `.cursor/skills -> ../.agents/skills` - Cursor compatibility symlink
- `.codex/config.toml` - Codex configuration

Install/update skills with the standard CLI:

```bash
skills add <owner/repo> --agent claude-code cursor -y
```

---

## Project: brain

Rust workspace (edition 2024). Platform-agnostic AI agent engine. A `Brain` struct
orchestrates five traits (Provider, Tool, Store, AgentLoop, Transport) into a
session-aware reasoning engine.

### Conventions
- Traits as interfaces, not class hierarchies
- Five core traits: Provider, Tool, Store, AgentLoop, Transport
- `Brain` in brain-core orchestrates all traits into a session-aware engine
- `thiserror` for library errors, `anyhow` for application code only
- `tracing` for logging, never `println!` in library code
- Library code never reads env vars — config is passed as data
- Greenfield project — prefer clean code over backward compatibility. Breaking
  changes are fine; confirm with user if unsure rather than adding compat shims.
- Install repo hooks with `lefthook install` after cloning if you want local
  pre-commit enforcement.
- NEVER run `cargo fmt --all` in this repo. It can reformat unrelated files and
  create noisy diffs outside the intended change scope. Format only the files
  you intentionally edited.
- The committed `lefthook` hook only formats staged Rust files and re-stages
  them. It must not run `cargo fmt --all`.
- Run `cargo test --workspace` before committing
- Strive for increased test coverage — add or update tests whenever making changes
- Use `just coverage` for an HTML coverage report or `just coverage-summary` for
  a quick text summary (requires `cargo-llvm-cov`)

### Crate Map
- `brain-types` — data structs + 5 traits (Provider, Tool, Store, AgentLoop, Transport)
- `brain-providers` — provider backends, each feature-gated in its own folder:
  - `mock` — MockProvider (always available)
  - `openai/` — OpenAiProvider, OpenAiConfig, OpenAiConfigPreset (feature `openai`, on by default)
  - `mistralrs/` — MistralRsProvider, MistralRsConfig, MistralRsModelPreset (feature `mistralrs`, off by default)
- `brain-stores` — InMemoryStore, FileStore (session CRUD + message persistence)
- `brain-tools` — tool implementations using driver trait + adapter pattern:
  - `echo.rs` — EchoTool (platform-independent, no driver)
  - `file_read/` — FileReadDriver, FileReadTool\<T\>, FileReadDriverNative (feature `native`)
  - `file_write/` — FileWriteDriver, FileWriteTool\<T\>, FileWriteDriverNative
  - `file_edit/` — FileEditDriver, FileEditTool\<T\>, FileEditDriverNative
  - `shell/` — ShellDriver, ShellTool\<T\>, ShellDriverNative
  - `glob_search/` — GlobDriver, GlobTool\<T\>, GlobDriverNative
  - `grep/` — GrepDriver, GrepTool\<T\>, GrepDriverNative
  - `native_tools()` — preset returning all native-backed tools
- `brain-loops` — SimpleLoop (agent loop orchestration)
- `brain-transports` — CliTransport (stdin/stdout)
- `brain-core` — Brain orchestration engine, ProviderRouter + re-exports all crates above
- `cli-echo` — example CLI using Brain + CliTransport + OpenAI-compatible providers
- `cli-local` — example CLI using Brain + CliTransport + MistralRsProvider (in-process GGUF)

### Specs
Canonical specs live in `openspec/specs/`. Check `openspec/changes/` for pending
proposals.

### Development Workflow

Use openspec for spec-driven development. Every non-trivial change goes through
three stages:

**Pre-code (scope the change):**
1. `openspec list` and `openspec list --specs` to understand current state
2. Create a change directory: `mkdir -p openspec/changes/<change-id>/specs/<capability>`
3. Write `proposal.md` (why + what), `design.md` (decisions + trade-offs), `tasks.md` (checklist)
4. Write delta specs under `specs/<capability>/spec.md` using ADDED/MODIFIED/REMOVED
5. `openspec validate <change-id> --strict` to verify format
6. Get approval before implementing

**Implementation:**
1. Read `proposal.md`, `design.md`, `tasks.md`
2. Implement tasks sequentially, mark `- [x]` as you go
3. `cargo build --workspace && cargo test --workspace` to verify
4. Check for stale references in related openspec docs

**Post-code (sync specs):**
1. Update any stale design/proposal docs to reflect what was actually built
2. `openspec validate <change-id> --strict`
3. `openspec archive <change-id> --yes` to promote delta specs to canonical `openspec/specs/`
4. `openspec validate --specs` to confirm all specs are clean
