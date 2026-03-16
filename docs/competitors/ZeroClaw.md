# ZeroClaw

## One-Line Take

`ZeroClaw` is the closest Rust comparison to `brain` in architectural spirit: real traits, a real iterative loop, and a broad runtime surface.

## Snapshot

- Vertical: agent runtime platform with models, tools, memory, channels, gateway, and dashboard surfaces.
- Best comparison inside `brain`: trait design, provider capabilities, runtime routing, and iterative tool loop behavior.
- Main lesson: `ZeroClaw` shows a plausible Rust path toward a broad runtime, but it still concentrates too much in one large root crate.

## Tool Call Method

`ZeroClaw` has a real `Tool` trait and concrete tool implementations. It also supports two dispatch modes depending on provider capability: native structured tool calls when available, and prompt/XML-guided tool execution when not.

This is a strong design for `brain` to study because it separates the tool abstraction from the provider's specific tool-call affordances instead of assuming one protocol shape.

## Provider / Model / Mode Method

The provider layer is rich. Providers declare capabilities such as native tool calling and vision, while higher-level routing can steer requests to different provider and model combinations. Aliases and fallbacks are also part of the core design.

This is one of the clearest external validations of `brain`'s provider trait direction. The main difference is that `ZeroClaw` bundles more policy and routing inside the main runtime crate than `brain` likely wants to.

## Agent Loop Method

`ZeroClaw` has a genuine iterative loop. It builds system context, loads memory, selects a model route, submits the conversation, parses tool calls, executes tools, feeds the results back, trims history, and continues up to a configured maximum iteration count.

This is stronger than most repos in this set as an `AgentLoop` benchmark. It is particularly relevant for how `brain` may want to balance provider routing, memory integration, and tool continuation inside one orchestrator.

## Permissions / Sandbox Method

The security policy is concrete and code-backed. The runtime supports autonomy levels, command risk classification, approval gates, path confinement, resolved-path checks, allowed roots, and execution-rate limits. Runtime adapters can execute commands natively or through Docker.

The caveat is that safety strength depends on configuration. Docker-backed execution is meaningfully stronger than native runtime execution, and some sandbox-specific code appears separate from the main execution path.

## Platform Support

`ZeroClaw` has explicit cross-platform intent in the repo:

- Linux
- macOS
- Windows
- Termux / mobile-adjacent environments

It also includes web/dashboard surfaces and additional workspace crates beyond the main runtime.

## Technical Architecture

This repo is still more centralized than `brain` should be, but it is far more aligned with `brain` than the product-heavy TypeScript platforms. It exposes real traits for provider, tool, memory, channel, tunnel, and runtime behavior, even if many implementations still live in one large crate.

The main takeaway is that `ZeroClaw` validates the direction of a trait-driven Rust agent runtime, while also showing the risks of letting that runtime accumulate too many responsibilities in one place.

## Server/Client Architecture

### Process Model

ZeroClaw is a **single-binary design**: CLI, gateway, and agent all compile into one binary. The interactive CLI runs the agent in-process with no server. The gateway is an optional HTTP/WebSocket layer on top of the same in-process runtime.

| Command | Server Location | Transport |
|---------|-----------------|-----------|
| `zeroclaw` (agent CLI) | Same process | In-process (stdin via `CliChannel` + `mpsc`) |
| `zeroclaw gateway start` | Same process | Axum HTTP/WebSocket/SSE |
| `zeroclaw daemon` | Same process | Gateway + channels + heartbeat + cron |

### Default Flow (`zeroclaw`)

1. `main.rs` → `Commands::Agent` → `agent::run(config, message, provider, model, ...)`
2. `CliChannel` reads stdin and sends to `tokio::sync::mpsc::Sender<ChannelMessage>`.
3. Agent loop (`run_tool_call_loop`) processes messages in the same process.
4. No IPC, no network, no server -- purely in-process.

### Gateway Mode

`zeroclaw gateway start` starts an Axum HTTP server:

- REST API: status, config, tools, memory, admin
- WebSocket: `GET /ws/chat` -- each connection creates an in-process `Agent` and calls `agent.turn(&content)` directly
- SSE: `GET /api/events` via `tokio::sync::broadcast::channel`
- Health: `GET /health`, `GET /metrics`
- Admin: `POST /admin/shutdown`
- Security: refuses public bind without tunnel or explicit `allow_public_bind` config

### Daemon Mode

`zeroclaw daemon` is the gateway plus long-running services (channel adapters, heartbeat, cron tasks). Can be installed as a systemd/launchd service via `zeroclaw service install`.

### Web Dashboard

The gateway serves an embedded web dashboard (via `rust-embed`). Clients connect over the browser to the gateway HTTP port for a web UI.

### Server Lifecycle

- **CLI agent**: Process exits when user quits.
- **Gateway/Daemon**: Long-running; exits on Ctrl+C or `POST /admin/shutdown`.
- **Persistence**: SQLite (`brain.db`), config, memory survive process exit. No agent process persistence.

### Client/Server Boundary

No formal client/server trait. The boundary is `AppState` (shared config, provider, memory, tools, event channels) passed to Axum handlers. Each WebSocket connection creates its own in-process `Agent`. The `Channel` trait abstracts messaging platforms (Telegram, Discord, etc.) but is separate from the gateway layer.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary with explicit provider capabilities and routing hooks.
- `Tool`: first-class boundary with real concrete built-ins.
- `Store`: first-class boundary through memory traits and concrete backends.
- `AgentLoop`: implicit boundary; the loop is strong and dedicated, but not exposed as a separate pluggable loop trait.
- `Transport`: first-class boundary through channel abstractions.

`ZeroClaw` is the strongest external validation that `brain`'s five-trait architecture is directionally right, even if its own runtime is still more centralized than ideal.

## Concrete Tool Implementation Notes

- `FileRead`: native Rust async filesystem implementation, with extra handling for formats like PDF.
- `FileWrite`: native Rust async filesystem implementation.
- `FileEdit`: native Rust read/replace/write flow.
- `Glob`: native Rust implementation using the `glob` crate.
- `Grep` / content search: shells out to `rg`, with fallback to `grep`, then post-processes output in Rust.
- `Shell` / `Bash`: shells out through runtime adapters, using platform-specific command invocation.

This makes `ZeroClaw` the closest direct comparison to `brain`'s current tool stack: native file tools, native globbing, and external CLI-backed content search.

## What To Steal

- Provider capability flags and routing.
- Iterative loop design that cleanly re-feeds tool results.
- Explicit runtime policy around path and command safety.

## What To Differentiate

- Keep the crate graph smaller and clearer instead of concentrating most behavior in one root crate.
- Make sandbox integration more obviously central than optional runtime choice.
- Preserve a more explicit separation between engine core and product shells.

## Data Model

### Sessions / Conversations

No dedicated session struct. The agent keeps an in-memory `Vec<ConversationMessage>` and uses `workspace_dir` as the scope. `MemoryEntry.session_id` scopes long-term memories to a session, but conversations themselves are not persisted.

### Messages

`ChatMessage { role: String, content: String }` for simple messages. `ConversationMessage` enum for multi-turn history: `Chat(ChatMessage)`, `AssistantToolCalls { text, tool_calls: Vec<ToolCall>, reasoning_content }`, `ToolResults(Vec<ToolResultMessage>)`. `ToolCall { id, name, arguments: String }`. All in-memory only.

### Memory

`MemoryEntry { id, key, content, category (Core/Daily/Conversation/Custom), timestamp, session_id, score }`. Stored in SQLite (`brain.db` at `workspace_dir/memory/brain.db`). Schema includes `memories` table with FTS5 full-text search and optional embedding BLOBs. `MEMORY_SNAPSHOT.md` for cold boot hydration. Memory is long-term facts/preferences, distinct from ephemeral conversation history.

### Credentials

`AuthProfile { id, provider, profile_name, kind (OAuth/Token), account_id, workspace_id, token_set, token, metadata, timestamps }`. `AuthProfilesData` holds `active_profiles` (provider -> profile_id map) and `profiles` (all profiles). Stored in `auth-profiles.json`, optionally encrypted with `SecretStore` (ChaCha20-Poly1305, key in `~/.zeroclaw/.secret_key`).

### Storage

Global config in `~/.zeroclaw/config.toml`. Workspace data in `workspace_dir/` (determined by `ZEROCLAW_WORKSPACE` env, `active_workspace.toml`, or config). No project entity beyond the workspace path.

## Key Evidence

- `repocache/zeroclaw-labs/zeroclaw/src/main.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/gateway/mod.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/gateway/ws.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/channels/cli.rs`
- `repocache/zeroclaw-labs/zeroclaw/README.md`
- `repocache/zeroclaw-labs/zeroclaw/Cargo.toml`
- `repocache/zeroclaw-labs/zeroclaw/src/lib.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/agent/agent.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/agent/dispatcher.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/providers/traits.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/providers/mod.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/providers/router.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/mod.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/file_read.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/file_write.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/file_edit.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/glob_search.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/content_search.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/shell.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/security/policy.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/runtime/native.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/runtime/docker.rs`
