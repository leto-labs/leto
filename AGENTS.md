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

## Git Worktrees

Agents should assume they may be running from `main` or from any linked worktree.

- Determine the active worktree root with `git rev-parse --show-toplevel`
- Determine the active branch with `git branch --show-current`
- Inspect all attached worktrees with `git worktree list`
- Treat the active worktree directory as the project root for commands, edits, tests, and relative paths

Worktree layout convention:

- The primary checkout is typically named `main`
- Linked worktrees should be sibling directories next to `main`, for example:
  - `../feature-xyz`
  - `../fix-shell-timeout`
- Prefer sibling worktrees over nested clones or ad hoc copies

Create new worktrees with sibling paths when branch isolation is useful:

```bash
git worktree add ../feature-xyz -b feature/xyz
git worktree add ../fix-shell-timeout -b fix/shell-timeout
```

If you are currently inside a linked worktree and need to create another one, use the repo root or any attached worktree:

```bash
git worktree list
git worktree add ../feature-xyz -b feature/xyz
```

Agent behavior in worktrees:

- Do not assume `main` is the active checkout; always verify the current worktree and branch first when context matters
- Keep branch-specific changes inside the current worktree only; never edit sibling worktrees from the active one
- Prefer worktree-local transient state such as `.env`, build artifacts, and agent session state unless a file is intentionally shared
- Suggest creating a fresh worktree when the user wants parallel agent work, risky experiments, or isolated feature development
- Before deleting a worktree, run `git status --short` inside it and `git worktree list` from any repo checkout

---

## Project: leto

Product name: `leto`

Core engine/workspace focus:

Rust workspace (edition 2024). Platform-agnostic AI agent engine. The current
application-facing stack centers on `agent-core`, `agent-runtime`,
`agent-store`, `agent-tools`, `agent-loops`, and the standalone `provider-*`
crates. Legacy `brain-*` source is kept only as a local gitignored archive
under `archive/brain/` when reference material is still needed.

### Conventions
- Traits as interfaces, not class hierarchies
- Five core traits: Provider, Tool, Store, AgentLoop, Transport
- `AgentCore` / `AgentCoreNative` in `agent-core` are the preferred
  application boundary
- `SessionEngine` in `agent-runtime` owns live per-session execution mechanics
- `Brain` in archived `brain-core` remains legacy engine infrastructure, not
  the primary product boundary
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
- Add Rust doc comments for new or changed public APIs, following rustdoc and general
  best practices so generated documentation stays useful over time
- Use `just coverage` for an HTML coverage report or `just coverage-summary` for
  a quick text summary (requires `cargo-llvm-cov`)

### Crate Map
- `provider` — shared v2 Provider SDK (request/event/capability/model metadata)
- `provider-openai` — standalone OpenAI-compatible v2 provider crate
- `provider-anthropic` — standalone Anthropic v2 provider crate
- `provider-mistralrs` — standalone mistral.rs local v2 provider crate (feature `mistralrs`, off by default)
- `provider-llamacpp` — standalone llama.cpp local v2 provider crate (feature `llamacpp`, off by default)
- `agent-runtime` — live session engine, runtime events, loop/runtime boundary, PTY and child-agent control
- `agent-store` — durable project/session/message/credential/trajectory storage, including `FileStore` and project bootstrap config helpers
- `agent-tools` — typed v2 tool implementations and the erased runtime-facing tool executor:
  - `echo.rs` — EchoTool (platform-independent, no driver)
  - `file_read/` — FileReadDriver, FileReadTool\<T\>, FileReadDriverNative (feature `native`)
  - `file_write/` — FileWriteDriver, FileWriteTool\<T\>, FileWriteDriverNative
  - `file_edit/` — FileEditDriver, FileEditTool\<T\>, FileEditDriverNative
  - `shell/` — ShellDriver, ShellTool\<T\>, ShellDriverNative
  - `glob_search/` — GlobDriver, GlobTool\<T\>, GlobDriverNative
  - `grep/` — GrepDriver, GrepTool\<T\>, GrepDriverNative
  - `native_tools()` — preset returning all native-backed tools
- `agent-loops` — current loop strategies such as `SimpleLoop`, `RobustLoop`, `Terminus2Loop`, and `TerminusKiraLoop`
- `agent-core` — shared product-facing core boundary (`AgentCore`, `AgentCoreNative`) over stores, providers, tools, and loops
- `agent-core-remote` — remote `AgentCore` client over the canonical hosted protocol
- `agent-server` — hosted server over the shared `AgentCore` boundary
- `agent-acp` — ACP adapter surface over `AgentCore`
- `agent-cli` — local CLI binary crate exposing the `agent` command
- `atif` — Harbor-compatible trajectory/export schema crate
- Local-only legacy archive: `archive/brain/crates/brain-*` is not committed
  and exists only for source reference during migration cleanup

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
