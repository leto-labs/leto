# Competitor Research

Source-first internal notes for projects adjacent to `brain`.

This folder focuses on the dimensions that matter most for `brain`'s architecture:

- tool call method
- provider / model / mode method
- agent loop method
- permissions / sandbox method
- platform support
- technical architecture

All requested projects are present in `repocache/repocache.json`, and the analysis below is based on the local clones in `repocache/`.

## Index

- [`OpenCode.md`](OpenCode.md)
- [`OpenClaw.md`](OpenClaw.md)
- [`Gastown.md`](Gastown.md)
- [`Rig.md`](Rig.md)
- [`AsyncOpenAI.md`](AsyncOpenAI.md)
- [`VercelAISDK.md`](VercelAISDK.md)
- [`PiMono.md`](PiMono.md)
- [`IronClaw.md`](IronClaw.md)
- [`ZeroClaw.md`](ZeroClaw.md)
- [`Codex.md`](Codex.md)

## Executive Summary

- `OpenCode` and `OpenClaw` are product platforms first. They solve far more than a reusable engine, which gives them depth in UX, integrations, and control-plane concerns, but also makes them less cleanly composable than `brain`.
- `Gastown` is best understood as an orchestration layer above coding agents, not as a direct engine peer. It is strongest where `brain` may eventually care about persistent coordination, work tracking, and agent operations.
- `Rig` is a strong Rust SDK/library comparison for provider, tool, and RAG composition, but it is not a full coding-agent runtime or control-plane platform.
- `async-openai` is best treated as an upstream OpenAI protocol and streaming reference for `brain-providers`, not as a full engine competitor.
- `Vercel AI SDK` is the strongest comparison for provider abstraction, streaming semantics, tool-call protocol design, and UI-facing transports. It is narrower than `brain` because it is not centered on durable session orchestration or a transport/store core.
- `pi-mono` is the closest TypeScript runtime analogue to `brain`: it has clear package layering from provider layer to agent loop to coding-agent product, but its abstractions are still package- and hook-centric rather than a small trait kernel.
- `ZeroClaw` is the closest Rust comparison in architectural spirit. It has real traits for core subsystems and a real iterative tool loop, even though most of the implementation still lives in one large root crate.
- `IronClaw` is strongest as a security-policy reference, but weaker as an architecture benchmark because some important seams appear more aspirational than fully wired in the current source.
- `Codex` is the strongest Rust product-level comparison. It has a large 70+ crate workspace with excellent sandboxing, MCP integration, and multi-platform coverage, but its central `codex-core` runtime concentrates too many responsibilities to serve as a reusable engine. It validates many of the same design choices `brain` is making (tool handler traits, JSONL session persistence, dotfile global storage) while showing the risks of a monolithic core.
- `brain` can differentiate by staying small and explicit around `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport`, while still borrowing good ideas from these larger systems.

## Feature Matrix

| Project | Vertical | Tool Call Method | Provider / Model / Mode | Agent Loop | Permissions / Sandbox | Platform Support | Architecture | Relevance To `brain` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `OpenCode` | Full AI coding product | Central registry builds built-ins, config tools, plugin tools, and MCP tools | Catalog-driven providers plus agent profiles that override model, tools, permissions, and params | Persistent session loop with streaming, subagents, compaction, retry, and doom-loop guards | Rule-based `allow` / `ask` / `deny`; host exec; no strong built-in sandbox | CLI/TUI, desktop, VS Code, HTTP server, JS SDK, web app | Monorepo modular monolith with one large runtime package | Great product benchmark; weak match for a minimal engine core |
| `OpenClaw` | Personal assistant and gateway platform | Typed tool definitions, delegated client tools, policy pipeline, plugin tools | Config-first model registry, fallback chains, plugin providers, per-agent model control | Serialized session run built around embedded `pi-*` runtime | Docker sandboxing, exec approvals, pairing, tool policies; plugins unsandboxed | Linux/macOS gateway, WSL, macOS app, iOS/Android nodes | Modular monolith centered on a long-running gateway | Strong control-plane benchmark; broader than a coding engine |
| `Gastown` | Advanced orchestration layer for external coding agents | Delegates coding tools to managed runtimes; adds hooks, mail, priming, nudges, and coordination around them | Runtime preset registry for agent CLIs such as Claude, Codex, Cursor, OpenCode, and pi; not a model-provider abstraction | Mayor / Convoy / Polecat orchestration with persistent work tracking, hooks, and handoffs | Mostly vendor yolo / bypass modes plus guard hooks and command allowlists; stronger sandboxing is future-oriented | macOS, Linux, Windows binaries; tmux-centric operational model | Go control-plane product with Beads, Dolt, git worktrees, tmux sessions, dashboard/feed | High relevance for orchestration, lower relevance for core provider/tool architecture |
| `Rig` | Rust LLM and agent SDK | First-class tool plumbing, tool traits, tool servers, and MCP adapters, but few built-in local coding tools | Strong provider/client abstraction plus vector-store and RAG composition | Concrete multi-turn agent implementation with hooks and max-turn control | Hook-based control, not a strong sandbox/runtime policy layer | Rust-native library workspace with CLI/Discord integrations and partial WASM story | Library-first Rust workspace centered on `rig-core` | Strong secondary benchmark for provider/tool design, weaker as a full five-trait peer |
| `async-openai` | OpenAI-specific Rust SDK | Typed OpenAI tool and hosted-tool schemas, but no local coding-tool runtime | Strong typed OpenAI config/client layer for HTTP, SSE, and realtime APIs | Not a reusable local agent loop; exposes primitives and examples | Delegates approvals, shell, search, and sandbox semantics to OpenAI APIs | Rust SDK with partial WASM support and Azure/OpenAI-compatible config | Protocol SDK workspace, not an engine | High relevance for `brain-providers/openai`, low relevance for full engine architecture |
| `Vercel AI SDK` | Provider and agent SDK | `tool()` / `dynamicTool()` plus local execution and provider-executed tool protocols | Strongest provider interface of the set; unified model factories and streaming APIs | Focused tool loop around `generateText()` / `streamText()` and `ToolLoopAgent` | Approval protocol exists; sandboxing delegated to external systems | Node plus React, Vue, Svelte, Angular, Next.js, LangChain integrations | Highly composable TS monorepo SDK | Best provider/tool API benchmark; narrower runtime scope |
| `pi-mono` | Layered agent toolkit and product family | Serializable tool schemas, auto tool execution, concrete coding-agent tools | Package-based provider registry and model registry with overrides and OAuth | Explicit loop in `pi-agent-core`, then session management and compaction in coding agent | Mostly hook- and app-layer approval; sandboxing varies by package | CLI, RPC, SDK, web UI, Slack bot, browser-capable packages | Clear package layering from SDK to runtime to products | Closest TypeScript runtime comparison |
| `IronClaw` | Security-first AI agent product | Intended provider-native function calling, but concrete registry wiring looks incomplete | Single provider trait plus preset aliases and a broad but uneven provider factory | Simple conversation loop; tool use appears single-pass | Strong policy design and sandbox backends, but integration is not always obvious | Mostly Unix-first from current source and docs | Monolithic Rust binary | Good security reference, weaker engine reference |
| `ZeroClaw` | Rust agent runtime OS / platform | Real `Tool` trait with native or prompt/XML dispatch depending on provider capability | Rich provider trait, capability flags, routing, aliases, and fallbacks | Real iterative tool loop with memory, routing, and configurable tool iterations | Strong command/path policy; runtime adapters; Docker isolation stronger than native | Linux, macOS, Windows, Termux | Large root crate with real traits and some workspace split | Closest Rust architecture benchmark today |
| `Codex` | Full AI coding product (OpenAI) | `ToolHandler` trait, `ToolRegistry`, `ToolRouter`; typed payloads; MCP tools; multi-agent tools | Registry-driven `ModelProviderInfo`; concrete `ModelClient`; Responses API only; config-first provider overrides | Session-driven loop via `CodexThread`; streaming, compaction, retry; JSONL rollout persistence | Strongest sandbox: Seatbelt (macOS), Landlock+seccomp (Linux), restricted tokens (Windows); approval gates | CLI, TUI, app server, MCP server, VSCode, remote WebSocket | 70+ crate Rust workspace; `codex-core` is dominant runtime | Strongest Rust product benchmark; validates tool/sandbox patterns |

## `brain` Baseline

Current local baseline for `brain`'s native tools:

- `FileRead`: native `tokio::fs::read_to_string` with line-number formatting
- `FileWrite`: native `tokio::fs::create_dir_all` plus `tokio::fs::write`
- `FileEdit`: native read / unique-match replace / write flow
- `Glob`: Rust `glob` crate
- `Grep`: native `walkdir` plus Rust `regex`; does not shell out to `rg`
- `Shell`: `tokio::process::Command` using `sh -c`

This matters because several competitors expose a similar tool surface but implement it very differently:

- some use native filesystem code
- some shell out to `rg`, `fd`, or the system shell
- some expose only provider-defined remote tools rather than local runtime tools

## Core Trait Matrix Against `brain`

Legend:

- `first-class`: clear top-level boundary in the repo
- `implicit`: real subsystem, but product-level rather than a clean reusable interface
- `missing`: not central or not evidenced as a real boundary

| Project | `Provider` | `Tool` | `Store` | `AgentLoop` | `Transport` | Read Against `brain` |
| --- | --- | --- | --- | --- | --- | --- |
| `brain` | first-class trait | first-class trait | first-class trait | first-class trait | first-class trait | Baseline target architecture |
| `OpenCode` | first-class | first-class | first-class, but concrete storage modules | implicit | implicit | Strong runtime package, weaker trait seams |
| `OpenClaw` | first-class | first-class | implicit | implicit | first-class | Strong gateway and tool boundaries, but loop/store are distributed |
| `Gastown` | implicit | missing | first-class | first-class | first-class | Excellent control-plane reference, weak direct match for provider/tool core |
| `Rig` | first-class | first-class | implicit | implicit | implicit | Strong Rust SDK benchmark for provider/tool composition |
| `async-openai` | first-class | first-class, but protocol-level | implicit | missing | missing | Best read as a provider SDK reference, not an engine peer |
| `Vercel AI SDK` | first-class | first-class | implicit | first-class | first-class | Excellent SDK seams, but store is not a core runtime concept |
| `pi-mono` | first-class | first-class | first-class, but concrete classes | first-class | first-class | Closest TS runtime analogue to `brain` |
| `IronClaw` | first-class | first-class, but skeletal | first-class | implicit | first-class via channels | Architecture vocabulary exists, but some seams are thinly realized |
| `ZeroClaw` | first-class | first-class | first-class | implicit | first-class via channels | Closest Rust trait-driven comparison |
| `Codex` | implicit (concrete `ModelClient`, no trait) | first-class (`ToolHandler` trait) | implicit (SQLite + JSONL, no trait) | implicit (embedded in `Codex`/`Session`) | implicit (app-server, TUI, exec modes) | Strong product, but few clean trait seams |

## Tool Implementation Matrix

Legend:

- `native`: implemented in repo code using local runtime or language APIs
- `shell-out`: implemented by spawning external tools such as `rg`, `fd`, or a shell
- `provider-defined`: repo defines the tool schema or adapter, but execution happens in the external provider runtime
- `declared only`: mentioned or modeled, but not clearly implemented as a concrete tool in the repo snapshot

| Project | `FileRead` | `FileWrite` | `FileEdit` | `Glob` / `Find` | `Grep` / content search | `Shell` / `Bash` |
| --- | --- | --- | --- | --- | --- | --- |
| `brain` | native Rust fs | native Rust fs | native Rust replace/write | native `glob` crate | native `walkdir` + `regex` | shell-out via `sh -c` |
| `OpenCode` | native Node/Bun fs | native Node/Bun fs | native edit plus diff helpers | shell-out to `rg` | shell-out to `rg` | shell-out via spawned shell |
| `OpenClaw` | native wrappers over upstream tool plus fs / sandbox bridge | native wrappers over upstream tool plus fs / sandbox bridge | native wrappers over upstream tool plus fs / sandbox bridge | not evidenced as local concrete tool | not evidenced as local concrete tool | native process control plus PTY and optional Docker exec |
| `Gastown` | delegated to external runtimes | delegated to external runtimes | delegated to external runtimes | not centralized as a GT tool | not centralized as a GT tool | delegated to external runtimes with GT guardrails around session behavior |
| `Rig` | delegated or app-authored | delegated or app-authored | delegated or app-authored | not centralized as agent tool; ingestion helpers exist | delegated or provider-hosted | delegated or app-authored |
| `async-openai` | not centralized as local tool | delegated to hosted APIs | delegated to hosted APIs | missing as generic built-in | delegated to hosted search APIs | delegated to hosted shell/container APIs |
| `Vercel AI SDK` | provider-defined | provider-defined | provider-defined | missing as generic built-in | missing as generic built-in | provider-defined |
| `pi-mono` | native Node fs | native Node fs | native Node edit logic | shell-out to `fd` in coding agent | shell-out to `rg` in coding agent | shell-out via spawned shell |
| `IronClaw` | declared only | declared only | missing or not central | missing as agent tool | missing as agent tool | declared only as tool; shell spawning exists elsewhere |
| `ZeroClaw` | native Rust fs | native Rust fs | native Rust replace/write | native `glob` crate | shell-out to `rg`, fallback to `grep` | shell-out via runtime adapter |
| `Codex` | native Rust fs (`tokio::fs`) | native via `apply_patch` | native `apply_patch` crate (tree-sitter + `similar`) | native `list_dir` (`tokio::fs::read_dir`) | shell-out to `rg` | shell-out via sandboxed shell with PTY |

## Tool Takeaways

- `OpenCode` and `pi-mono` expose a very similar user-facing coding-agent tool surface, but both rely on external command-line tools for search-heavy primitives. In both cases, `grep` is effectively `rg` orchestration rather than a pure in-process search engine.
- `ZeroClaw` is the closest external match to `brain`'s current tool philosophy for file operations: read, write, edit, and glob are native runtime tools, while full-text search still shells out to `rg`.
- `Rig` is useful for comparing tool abstractions and MCP/vector-store integration, but it does not try to ship a built-in local coding-tool suite like `brain`, `pi-mono`, or `ZeroClaw`.
- `async-openai` is useful one layer lower than the rest: it models hosted tool schemas and streaming events very well, but it is not a local tool runtime at all.
- `Vercel AI SDK` is not a strong reference for local file/search tool implementation because most comparable tools are provider-defined remote tools rather than local runtime tools.
- `OpenClaw` is somewhat deceptive if looked at only from prompts and docs: the product exposes coding-style tools, but not all of those tools are implemented locally in the repo. Some are inherited from the embedded `pi-*` runtime.
- `Gastown` should be read separately from the others: it is not trying to own file/search/shell tools at all. It orchestrates runtimes that already have those tools, then adds persistent tracking, mail, nudges, hooks, and role-based control on top.
- `IronClaw` currently looks more like a declared tool architecture than a fully realized concrete tool stack.
- `Codex` is the closest Rust product to `brain` in tool philosophy: native file read, tree-sitter-powered patch application, and `rg`-backed search. It also demonstrates the strongest sandboxing story of the set, with platform-specific implementations (Seatbelt, Landlock, Windows restricted tokens) that `brain` should study.

## Storage Layout Matrix

How each framework persists data to disk. This matters for `brain`'s decision on global vs. project-local storage and the nesting of sessions, messages, and credentials.

| Project | Global Root | Root Method | Credentials | Sessions Format | Messages Location | Project-Local | Config Location |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `brain` (proposed) | `~/.brain/` | Hardcoded dotfile | `~/.brain/credentials/{provider}/{id}.json` | JSON metadata + JSONL messages per session | Nested inside session dir as `messages.jsonl` | `.agents/config.toml` | Global: `~/.brain/config.toml`; Project: `.agents/config.toml` |
| `OpenCode` | `~/.local/share/opencode/` (data); `~/.config/opencode/` (config) | XDG (`xdg-basedir`); `OPENCODE_CONFIG_DIR` env override | `auth.json` in data dir | SQLite (`opencode.db`) | In SQLite (`SessionTable`, `MessageTable`, `PartTable`) | `.opencode/` (agents, commands, plugins, skills, tools, modes, themes); `opencode.json` in project root | Global: `~/.config/opencode/opencode.json`; Project: `opencode.json`, `.opencode/` |
| `OpenClaw` | `~/.openclaw/` | Hardcoded dotfile | `.env` in `~/.openclaw/` | In-memory / plugin-driven; `memory/` for conversation memory | In `memory/`; format depends on memory plugin (e.g. LanceDB) | `~/clawd/` workspace | Global: `~/.openclaw/openclaw.json` (JSON5) |
| `ZeroClaw` | `~/.zeroclaw/` (config); workspace dir (data) | `ZEROCLAW_WORKSPACE` env → `active_workspace.toml` → `~/.zeroclaw/config.toml` | `auth-profiles.json` (ChaCha20-Poly1305 encrypted); key in `~/.zeroclaw/.secret_key` | SQLite (`brain.db` in workspace `memory/`) | In SQLite (`memories` table; hybrid vector + keyword); `MEMORY_SNAPSHOT.md` for cold boot | `workspace_dir/` with `memory/`, `MEMORY_SNAPSHOT.md` | Global: `~/.zeroclaw/config.toml`; Workspace: `workspace_dir/config.toml` |
| `IronClaw` | XDG via `directories` crate | `directories::ProjectDirs` (XDG) | PostgreSQL or LibSQL; AES-256-GCM; OS keychain for master key | PostgreSQL/LibSQL | Provider/DB-dependent | Not clearly documented | XDG config dirs |
| `pi-mono` | `~/.pi/` or project-local | `piConfig.configDir` = `.pi` | In `.pi/`; API keys via env or config | JSON/JSONL in `.pi/` | In session files | `.pi/` in project root | `.pi/` (combined config + data) |
| `Codex` | `~/.codex/` | `$CODEX_HOME` env → `dirs::home_dir()/.codex` | `auth.json` (file mode), OS keyring, or `secrets/local.age` (age-encrypted) | JSONL rollouts (`sessions/rollout-*.jsonl`); SQLite (`state_5.sqlite`) for thread metadata | Inline in JSONL rollout (one event per line in session file) | `.codex/config.toml`, `.codex/skills/`, `.codex/rules/` | System: `/etc/codex/config.toml`; User: `~/.codex/config.toml`; Project: `.codex/config.toml` |

### Storage Takeaways

- **Majority pattern is a single dotfile directory**, not XDG. Five of seven frameworks (`brain`, `OpenClaw`, `ZeroClaw`, `pi-mono`, `Codex`) use `~/.name/`. Only `OpenCode` and `IronClaw` use XDG.
- **SQLite is the most common session backend** (`OpenCode`, `ZeroClaw`, `Codex` metadata). JSONL is used by `Codex` for session rollouts and by `brain` (proposed) for messages.
- **Messages are almost always co-located with sessions**, either in the same SQLite tables (`OpenCode`, `ZeroClaw`) or in the same JSONL file (`Codex`). `brain`'s proposed `messages.jsonl` nested inside a session directory follows the same co-location principle.
- **Credentials vary widely**: plain JSON (`OpenCode`, `Codex`), dotenv (`OpenClaw`), encrypted vault (`ZeroClaw`, `IronClaw`), OS keyring (`Codex` optional). `brain`'s proposed `credentials/{provider}/{id}.json` is closest to `Codex`'s `auth.json` approach.
- **Project-local config is universal**: every framework supports a project-level config file or directory. The naming convention varies (`.opencode/`, `.codex/`, `.pi/`, `.agents/`), but the pattern is consistent.
- **`Codex` has the most layered config precedence**: system → user → project → CLI flags. Most others use just user → project.
- **Environment variable overrides are common** for the global root (`$CODEX_HOME`, `$ZEROCLAW_WORKSPACE`, `$OPENCODE_CONFIG_DIR`). `brain` should support a `$BRAIN_HOME` override.

## Data Model Matrix

Actual data structures each framework uses for projects, sessions, messages, and credentials. Based on source code inspection of repocache clones.

### Project Entity

| Project | Has Project? | Key Fields | Identification | Notes |
| --- | --- | --- | --- | --- |
| `brain` | Yes: `Project` struct | id (ULID), name, root (PathBuf), config, timestamps | ULID | Clean entity linking sessions to a working directory |
| `OpenCode` | Yes: `ProjectTable` | id, worktree, vcs, name, icon, sandboxes, commands, timestamps | String ID | Richest project model; includes VCS, icons, sandbox config |
| `Codex` | No | N/A | `cwd` on each thread | Threads store `cwd`, `git_branch`, `git_sha` directly |
| `pi-mono` | No | N/A | `cwd` in session header | Sessions keyed by encoded cwd path on disk |
| `OpenClaw` | Sort of: `AgentConfig` | id, name, workspace, model, skills, sandbox, tools | Agent ID | Agent is the organizational unit, not project |
| `ZeroClaw` | Sort of: `workspace_dir` | Just a path | Path | No first-class struct; workspace scopes memory |
| `IronClaw` | No | N/A | N/A | Config-only; no project concept |

### Session Entity

| Project | Key Fields | Persistence | Notable Extras |
| --- | --- | --- | --- |
| `brain` | id (ULID), project_id, title, timestamps | JSON file | Minimal and clean |
| `OpenCode` | id, project_id, workspace_id, parent_id, slug, directory, title, version, share_url, summary (additions/deletions/files/diffs), revert, permission ruleset, compacting/archived timestamps | SQLite row | Fork, compaction, git summary, permissions (~20 fields) |
| `Codex` | id (UUID v7), rollout_path, cwd, source, model_provider, title, sandbox_policy, approval_mode, tokens_used, git info, archived_at, agent info | SQLite metadata + JSONL rollout | Thread metadata mirrors JSONL header (~20 fields) |
| `pi-mono` | id, cwd, parentSession, name, timestamps (from file) | JSONL header line | Parent session for forks |
| `OpenClaw` | sessionId, agentId, updatedAt, model/provider overrides, token counts, compaction count, label, channel, queueMode, ~40 more fields | JSON index (`sessions.json`) | Extremely rich metadata (~50 fields) |
| `ZeroClaw` | Implicit `session_id` in memory entries | No dedicated session store | Session is just a scope for memories |
| `IronClaw` | id (UUID v4), turn_count, max_turns | In-memory only | No persistence |

### Message Entity

| Project | Key Fields | Persistence | Tool Call Representation |
| --- | --- | --- | --- |
| `brain` | id (ULID), role, content, tool_calls, tool_call_id, created_at | JSONL (append-only) | `Vec<ToolCall>` on assistant message + separate `Tool` role messages for results |
| `OpenCode` | Message (id, session_id, data) + Part (id, message_id, data) | SQLite (two tables) | Parts with type variants: text, reasoning, file, tool, snapshot, patch, agent, compaction |
| `Codex` | `RolloutItem` enum: SessionMeta, ResponseItem, Compacted, TurnContext, EventMsg | JSONL (one item per line) | `FunctionCall`, `LocalShellCall`, `CustomToolCall` as ResponseItem variants |
| `pi-mono` | UserMessage, AssistantMessage, ToolResultMessage (union type) | JSONL (as SessionEntry tree with parentId) | `ToolCall` in AssistantMessage.content + separate ToolResultMessage |
| `OpenClaw` | Same as pi-mono (uses pi-coding-agent) | JSONL | Same as pi-mono |
| `ZeroClaw` | `ConversationMessage` enum: Chat, AssistantToolCalls, ToolResults | In-memory only | Separate enum variant for tool calls with `Vec<ToolCall>` |
| `IronClaw` | id (UUID v4), role, content, tool_calls, tool_results, content_blocks, timestamp | In-memory only | On message struct directly; `ContentBlock` enum for multimodal |

### Credential Entity

| Project | Key Fields | Persistence | Multi-Provider | Health Tracking |
| --- | --- | --- | --- | --- |
| `brain` | id, credential (ApiKey/OAuth), health, enabled, created_at | JSON per provider dir | Yes: `credentials/{provider}/{id}.json` | Yes: consecutive errors, last_ok, last_error |
| `OpenCode` | id, email, url, access_token, refresh_token, token_expiry | SQLite (`AccountTable`) | Single active account | No |
| `Codex` | auth_mode, openai_api_key, tokens (id_token, access_token, refresh_token), last_refresh | Single `auth.json` or keyring | Single provider (OpenAI) | No |
| `pi-mono` | ApiKeyCredential or OAuthCredential | `auth.json` (flat dict by provider) | Yes: keyed by provider name | No |
| `OpenClaw` | AuthProfileConfig (provider, mode, email) + SecretRef (env/file/exec sources) | Config + `.env` + secret providers | Yes: profiles with ordering and cooldowns | No (but cooldowns) |
| `ZeroClaw` | AuthProfile (id, provider, kind, token_set, metadata, timestamps) in AuthProfilesData | `auth-profiles.json` (optionally ChaCha20-Poly1305 encrypted) | Yes: profiles per provider with active selection | No |
| `IronClaw` | api_key in ProviderConfig | Config file / env vars | Minimal | No |

### Data Model Takeaways

- **`brain` has the right level of abstraction.** Its Project, Session, Message, and CredentialEntry types are cleaner than the competition without being too minimal. No changes needed.
- **`brain` is the only framework with per-credential health tracking.** This is a genuine differentiator for reliability.
- **Message format is validated.** The `{ role, content, tool_calls, tool_call_id }` shape matches pi-mono and IronClaw exactly. JSONL persistence matches Codex and pi-mono.
- **Project as a named entity is rare but valuable.** Most frameworks use bare `cwd`. `brain`'s Project with `root: Option<PathBuf>` gives a stable identity to link sessions to.
- **Session metadata should stay lean.** OpenCode and OpenClaw prove that session metadata can bloat dramatically. `brain`'s 5-field Session is intentional.
- **OpenCode's Message+Part split is overkill** for `brain`. It exists to support rich UI rendering of diffs, patches, and snapshots. `brain`'s flat Message list is simpler and sufficient.
- **Codex's RolloutItem enum** (events, not just messages) is interesting for future consideration but adds complexity. `brain`'s append-only Message JSONL is simpler and covers the same ground.

## Suggested Next Analyses

- `mastra`: useful if you want another SDK-first comparison with stronger workflow/app-builder positioning.
- `langgraph`: useful if you want a graph-oriented agent-loop benchmark rather than a coding-agent benchmark.

## Working Conclusion

If the goal is to sharpen `brain`'s engine boundaries, the most useful primary comparisons are:

- `ZeroClaw` for Rust trait design and runtime composition
- `Codex` for Rust product-level tool/sandbox patterns and session persistence
- `pi-mono` for layered agent-loop packaging
- `Vercel AI SDK` for provider and tool-call API design

Secondary but still important references are:

- `Rig` for Rust provider/tool/RAG library design
- `async-openai` for OpenAI wire formats, streaming events, and hosted tool schemas

If the goal is to pressure-test product direction beyond the core engine, the most useful primary comparisons are:

- `Codex` for full-stack Rust agent product with MCP, sandboxing, and multi-platform TUI/IDE/app-server
- `OpenCode` for coding-agent UX and session orchestration
- `OpenClaw` for gateway, approvals, nodes, and multi-surface deployment
- `Gastown` for persistent multi-agent orchestration and work tracking
- `IronClaw` for security policy ambitions
