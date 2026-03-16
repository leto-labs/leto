# Codex

## One-Line Take

`Codex` is a full-stack Rust AI coding agent with strong sandboxing, MCP integration, and broad platform coverage, but a tightly integrated core.

## Snapshot

- Vertical: end-user AI coding product spanning CLI, TUI, desktop app server, MCP server, and IDE integration (VSCode).
- Best comparison inside `brain`: product pressure across all five traits — `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport`.
- Main lesson: a 70+ crate Rust workspace can deliver a polished product experience, but the central `codex-core` runtime concentrates too many responsibilities to serve as a reusable engine.

## Tool Call Method

`Codex` defines tools as JSON Schema specs and implements them via a `ToolHandler` trait:

```rust
#[async_trait]
pub trait ToolHandler: Send + Sync {
    type Output: ToolOutput + 'static;
    fn kind(&self) -> ToolKind;  // Function | Mcp
    async fn is_mutating(&self, invocation: &ToolInvocation) -> bool;
    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError>;
}
```

A `ToolRegistryBuilder` builds specs and registers handlers via `build_specs_with_discoverable_tools()`. A separate `ToolRouter` maps model response items (FunctionCall, ToolSearchCall, LocalShellCall, CustomToolCall, Mcp) to dispatched `ToolCall` values. Tool payloads are variant-typed: `ToolPayload::Function`, `ToolPayload::Mcp`, `ToolPayload::LocalShell`, `ToolPayload::ToolSearch`, `ToolPayload::Custom`.

Results are turned into `ResponseInputItem` and fed back into the session. Mutating tools wait on a `tool_call_gate` before running, and `after_tool_use` hooks can abort on failure.

Built-in tools include: `exec_command`, `write_stdin`, `shell`, `read_file`, `grep_files`, `list_dir`, `apply_patch`, MCP resource tools, `update_plan`, `view_image`, `request_user_input`, `request_permissions`, `tool_search`, `tool_suggest`, `artifacts`, `js_repl`, and multi-agent tools (`spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`).

## Provider / Model / Mode Method

Provider support is registry-driven via `ModelProviderInfo` with built-in entries for `openai`, `ollama`, and `lmstudio`. The config file can extend or override these with `base_url`, `env_key`, `wire_api`, retry, and timeout settings. All providers use the Responses API wire format (`wire_api = "responses"`).

Model selection flows through a `ModelsManager` that resolves from config, CLI flags, and session state. `ModelInfo` carries per-model metadata including `shell_type`, `apply_patch_tool_type`, and `web_search_tool_type`.

There is no explicit `Provider` trait; `ModelClient` is a concrete type that wraps HTTP/WebSocket calls to the Responses API. The `codex-connectors`, `lmstudio`, and `ollama` crates provide adapter layers.

Modes exist as config profiles and approval/sandbox policies. Agent roles (via `spawn_agent`) can override model and reasoning effort.

## Agent Loop Method

The loop is driven by `CodexThread` which wraps the `Codex` runtime. `submit()`, `steer_input()`, and `next_event()` drive turn progression. The `Session` struct holds conversation state, `ModelClient`, `ToolRouter`, MCP connections, and hooks.

Turn flow: new turn → stream model output → handle `ResponseItem` variants (text, function calls, tool_search, local_shell, custom, MCP) → dispatch tools → append results → repeat.

Streaming uses the Responses API over WebSocket or SSE via `ResponseStream` / `ResponseEvent`. Retries are configurable at both HTTP level (`request_max_retries`) and stream level (`stream_max_retries`, `stream_idle_timeout`).

Compaction is supported through a dedicated `compact` module with `run_inline_auto_compact_task` and remote compaction via `CompactClient`. Session persistence uses JSONL rollout files, with SQLite (`state_5.sqlite`) for thread metadata and backfill.

## Permissions / Sandbox Method

Codex has the strongest sandboxing of any framework in this set. `SandboxPolicy` defines four levels: `ReadOnly`, `WorkspaceWrite`, `DangerFullAccess`, and `ExternalSandbox`.

Platform-specific implementations:
- **macOS**: Seatbelt profiles with `create_seatbelt_command_args_for_policies_with_extensions`.
- **Linux**: Dedicated `codex-linux-sandbox` binary using Landlock LSM and seccomp, with bubblewrap fallback.
- **Windows**: Restricted token sandbox via `codex-windows-sandbox`.

Execution permissions are modeled as `SandboxPermissions` (`UseDefault`, `WithAdditionalPermissions`, `RequireEscalated`). The `request_permissions` tool allows runtime permission escalation through the approval flow. `AskForApproval` and `GuardianApprovalRequest` gate mutating tool calls.

## Platform Support

- CLI: `codex` (interactive), `codex exec` (non-interactive batch mode).
- TUI: `codex-tui`, `codex-tui-app-server` (app-server-backed terminal UI).
- App server: `codex app-server` (stdio or WebSocket protocol for IDE integration).
- MCP server: `codex mcp-server` mode.
- IDE: VSCode integration via the app server protocol.
- Remote: `--remote ws://...` for remote TUI connections.

## Technical Architecture

The repo is a 70+ crate Rust workspace. `codex-core` is the dominant runtime, pulling in tools, sandboxing, connectors, MCP, and session management. Dedicated crates exist for `apply-patch` (tree-sitter), `file-search` (nucleo/ignore), `linux-sandbox` (Landlock/seccomp), and `windows-sandbox`.

The binary is `codex` from `cli/src/main.rs`, with additional binaries for `codex-exec`, `codex-app-server`, `codex-mcp-server`, and `codex-linux-sandbox`.

The architecture is more modular than a monolith but less trait-driven than `brain` aims to be. Tools and MCP are pluggable via the `ToolHandler` trait, but provider, loop, and store are concrete implementations embedded in `core`.

## Mapping To `brain` Core Traits

- `Provider`: no explicit trait; `ModelProviderInfo` + concrete `ModelClient` + HTTP/WebSocket adapters in `codex-api`/`codex-connectors`.
- `Tool`: `ToolHandler` trait, `ToolRegistry`, `ToolRouter`; closest match to `brain`'s `Tool` trait.
- `Store`: `state_db` (SQLite), `rollout` (JSONL); no `Store` trait abstraction.
- `AgentLoop`: implicit in `Codex`/`Session`; turn handling, streaming, tool dispatch; not a standalone interface.
- `Transport`: `app-server` (stdio/WebSocket), `tui`, `exec`; no `Transport` trait abstraction.

## Concrete Tool Implementation Notes

- `read_file`: native `tokio::fs::File` with `BufReader`, slice and indentation modes. No shell-out.
- `grep_files`: shell-out to `rg` (ripgrep) with `--files-with-matches`, `--sortr=modified`.
- `list_dir`: native `tokio::fs::read_dir`, recursive with depth limit.
- `apply_patch`: dedicated `codex-apply-patch` crate using tree-sitter for bash and `similar` for diffs.
- `shell` / `shell_command`: spawns user shell (bash/powershell) via `ShellCommandHandler`; sandboxed.
- `exec_command` / `write_stdin`: `UnifiedExecHandler` with PTY via `codex-utils-pty`; `UnifiedExecProcessManager`; sandboxed.

No separate glob tool; `grep_files` covers file search and `list_dir` covers directory listing.

## Storage Layout

### Global (`~/.codex/` or `$CODEX_HOME`)

```
~/.codex/
├── config.toml              # user config (TOML)
├── auth.json                # CLI auth (API key / OAuth token)
├── .credentials.json        # MCP OAuth (file mode)
├── secrets/
│   └── local.age            # age-encrypted secrets
├── sessions/
│   └── rollout-*.jsonl      # active session rollouts (JSONL)
├── archived_sessions/
│   └── rollout-*.jsonl      # archived sessions
├── memories/
│   ├── rollout_summaries/
│   ├── raw_memories.md
│   ├── MEMORY.md
│   ├── memory_summary.md
│   └── skills/
├── log/                     # log files
├── state_5.sqlite           # thread metadata, backfill, agent jobs
├── logs_1.sqlite            # tracing logs (10 MiB/partition, 10-day retention)
└── history.jsonl            # optional history
```

### Project-local (`.codex/`)

```
.codex/
├── config.toml              # project config (trust, sandbox)
├── skills/                  # project skills
└── rules/                   # project rules
```

Config precedence: `/etc/codex/config.toml` → `~/.codex/config.toml` → `.codex/config.toml` → CLI flags.

Auth supports multiple modes: `file` (default), `keyring` (OS keychain), `auto`, `ephemeral` (in-memory).

## Data Model

### Sessions / Threads

Codex uses "thread" for persisted conversations and "session" for in-memory runtime state. `ThreadMetadata` (SQLite `threads` table): id (UUID v7), rollout_path, created_at, updated_at, source, agent_nickname, agent_role, model_provider, cwd, cli_version, title, sandbox_policy, approval_mode, tokens_used, first_user_message, archived_at, git_sha, git_branch, git_origin_url. No explicit project entity -- threads store `cwd` and git info directly.

### Messages / Events

JSONL rollout files (`sessions/rollout-*.jsonl`). Each line is a `RolloutItem` enum: `SessionMeta` (header with id, cwd, source, agent info, git info), `ResponseItem` (the actual messages), `Compacted`, `TurnContext`, `EventMsg`. `ResponseItem` variants include: Message (role, content), Reasoning, FunctionCall, FunctionCallOutput, LocalShellCall, CustomToolCall, ToolSearchCall, WebSearchCall, ImageGenerationCall, GhostSnapshot, Compaction.

### Credentials

`AuthDotJson` at `$CODEX_HOME/auth.json`: auth_mode, openai_api_key, tokens (TokenData with id_token, access_token, refresh_token, account_id), last_refresh. Single-provider model (OpenAI only). Alternative storage modes: OS keyring, age-encrypted secrets (`secrets/local.age`), ephemeral (in-memory).

### State DB

SQLite (`state_5.sqlite`): tables for threads, logs, thread_dynamic_tools, memories, backfill_state, agent_jobs. Thread metadata is extracted from JSONL rollout headers and mirrored into SQLite for listing and search.

## What To Steal

- `ToolHandler` trait pattern with typed payloads and `is_mutating` gate.
- Platform-specific sandboxing (Seatbelt, Landlock/seccomp, Windows restricted tokens).
- MCP server and resource integration as first-class tool types.
- Session persistence via JSONL rollouts with SQLite metadata.
- Compaction (inline and remote) for long conversations.
- Permission escalation flow via `request_permissions` tool.
- Multi-agent spawning pattern (`spawn_agent`, `wait_agent`, `close_agent`).

## What To Differentiate

- Keep Provider, Store, AgentLoop, Transport as explicit swappable traits instead of concrete types embedded in one core runtime.
- Prefer a smaller core with clearer crate boundaries instead of a 70+ crate workspace.
- Use native grep/glob implementations where possible instead of shelling out to `rg`.
- Avoid coupling engine core to TUI, app server, and IDE concerns.
- Use a single `~/.brain/` dotfile root instead of XDG split (matching the majority pattern in the ecosystem).

## Key Evidence

- `repocache/openai/codex/codex-rs/Cargo.toml`
- `repocache/openai/codex/codex-rs/cli/src/main.rs`
- `repocache/openai/codex/codex-rs/core/src/lib.rs`
- `repocache/openai/codex/codex-rs/core/src/codex.rs`
- `repocache/openai/codex/codex-rs/core/src/codex_thread.rs`
- `repocache/openai/codex/codex-rs/core/src/client.rs`
- `repocache/openai/codex/codex-rs/core/src/model_provider_info.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/mod.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/registry.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/router.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/spec.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/handlers/read_file.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/handlers/grep_files.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/handlers/unified_exec.rs`
- `repocache/openai/codex/codex-rs/core/src/sandboxing/mod.rs`
- `repocache/openai/codex/codex-rs/core/src/mcp_connection_manager.rs`
- `repocache/openai/codex/codex-rs/core/src/auth/storage.rs`
- `repocache/openai/codex/codex-rs/utils/home-dir/src/lib.rs`
