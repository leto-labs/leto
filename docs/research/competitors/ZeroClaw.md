# ZeroClaw

## Overview

`ZeroClaw` is the closest Rust architecture comparison to `brain`: it has real subsystem traits, a real iterative tool loop, and concrete memory/routing infrastructure, even though too much still lives in one root crate.

| Item | Value |
| --- | --- |
| Vertical | Rust agent runtime platform with providers, tools, memory, channels, gateway, and dashboard surfaces |
| Best comparison inside `brain` | Provider capability routing, iterative tool looping, and trait-driven runtime structure |
| Main lesson | `ZeroClaw` validates the five-trait direction, while also showing how fast one crate can grow once routing, memory, and product shell concerns accumulate |

## Architecture

`ZeroClaw` keeps most of its implementation in one large `src/` tree, but the architecture vocabulary is strong: providers, tools, memory, runtime, channels, gateway, hooks, and approval each have distinct modules. `agent/loop_.rs` is the operational center for the interactive/runtime path, while `agent/prompt.rs` defines prompt sections, `providers/reliable.rs` wraps failover and retries, and `memory/` plus `runtime/` back the durable and execution-side subsystems.

Compared with `brain`, this is a trait-rich runtime that still has a product-shell gravity problem. It proves the design direction, but also proves how much discipline is needed to keep the central loop from becoming a catch-all.

## Agent Loop

The core loop is iterative. `run_tool_call_loop(...)` in `agent/loop_.rs` repeatedly calls the provider, parses native or fallback text-based tool calls, executes tools, appends tool results to history, and re-queries until either there are no more tool calls or `max_tool_iterations` is reached. This is a genuine re-query loop, not single-pass tool handling.

Streaming is partial and channel-oriented rather than full SSE event modeling. The main loop can stream progress and final response chunks through an optional `tokio::sync::mpsc::Sender<String>`, emitting short progress messages such as "Thinking..." and then streaming the final text in whitespace-bounded chunks. Provider-side streaming exists in `providers/reliable.rs`, but the core agent loop still uses `provider.chat(...)` for the main request path rather than making streaming the primary orchestration API.

Tool dispatch is adaptive. `run_tool_call_loop(...)` computes `tool_specs`, checks `provider.supports_native_tools()`, and either sends structured tool schemas or falls back to parsing tool calls out of text/XML-style output. When multiple tool calls are returned and interactive approval is not blocking, the loop can execute them concurrently. Ordered result tests in `agent/loop_.rs` confirm that multi-tool turns preserve deterministic result ordering even when execution is parallelized.

Compaction and trimming are handled directly in the agent module. `trim_history(...)` preserves the system prompt while dropping older non-system messages, and `auto_compact_history(...)` summarizes older turns into a bullet summary when the message threshold is exceeded. The compaction summarizer uses its own static instruction string and replaces the compacted span with a summary message, while `memory/snapshot.rs` separately manages durable memory cold-boot snapshots.

Retry and failure recovery are strongest in the provider wrapper. `providers/reliable.rs` implements a three-level strategy: retry the same provider/model pair with exponential backoff, rotate auth on retryable rate limits, then fall back across provider/model chains. It parses `Retry-After`, distinguishes non-retryable business rate limits from transient 429s, and supports streaming on the first provider that can do it. Inside the loop, cancellation is checked on every iteration and provider capability mismatches are surfaced explicitly.

Subagents exist, but they are not as first-class as Codex or OpenCode child threads. The `delegate` tool in `tools/delegate.rs` spins a subtask through `run_tool_call_loop(...)`, prepends delegation context to the prompt, and explicitly blocks infinite delegation depth. This is closer to delegated loop reuse than to a separate persisted child-session architecture.

Max-turn control is explicit. `run_tool_call_loop(...)` enforces `DEFAULT_MAX_TOOL_ITERATIONS = 10` when the configured value is zero, and the interactive path also trims history to `DEFAULT_MAX_HISTORY_MESSAGES` unless compaction is enabled. This makes ZeroClaw one of the clearer repos in the set for concrete loop guards.

Key types and functions:

- `run_tool_call_loop(...)` in `repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs`
- `trim_history(...)` and `auto_compact_history(...)` in `repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs`
- `SystemPromptBuilder` in `repocache/zeroclaw-labs/zeroclaw/src/agent/prompt.rs`
- `ReliableProvider` logic in `repocache/zeroclaw-labs/zeroclaw/src/providers/reliable.rs`
- `DelegateTool` in `repocache/zeroclaw-labs/zeroclaw/src/tools/delegate.rs`
- `export_snapshot(...)` and `hydrate_from_snapshot(...)` in `repocache/zeroclaw-labs/zeroclaw/src/memory/snapshot.rs`

## System Prompt & Prompt Building

The system prompt is hardcoded through composable prompt sections rather than one giant text blob. `agent/prompt.rs` defines `PromptSection`, `PromptContext`, and `SystemPromptBuilder`, with a default section set of `IdentitySection`, `ToolsSection`, `SafetySection`, `SkillsSection`, `WorkspaceSection`, `DateTimeSection`, `RuntimeSection`, and `ChannelMediaSection`.

Dynamic prompt assembly is one of ZeroClaw's clearest strengths. `IdentitySection` injects workspace files including `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, `BOOTSTRAP.md`, and `MEMORY.md`. `ToolsSection` renders every tool name, description, and JSON schema. `SkillsSection` injects skill instructions according to the configured prompt-injection mode, and the runtime/date/workspace sections append environment metadata.

This is also one of the most explicit repos in the set about bootstrap file limits. `BOOTSTRAP_MAX_CHARS` is `20_000` per file, and truncated files receive an explicit marker telling the model to use `read` for the full content. That is a direct context-window management strategy rather than an implicit convention.

There is no generalized prompt-template engine analogous to Codex custom prompts or pi-mono prompt templates. The prompt system is code-composed sections plus identity/bootstrap files and skills injection. That is simpler, but less user-extensible.

Message history formatting is conventional chat history, with the system prompt assembled separately from `PromptContext` and older turns trimmed or compacted before the next call. Native-tool providers get structured tool schemas, while non-native providers rely on prompt-guided tool invocation rules.

Key types and functions:

- `PromptContext` and `PromptSection` in `repocache/zeroclaw-labs/zeroclaw/src/agent/prompt.rs`
- `SystemPromptBuilder::with_defaults/build` in `repocache/zeroclaw-labs/zeroclaw/src/agent/prompt.rs`
- `inject_workspace_file(...)` in `repocache/zeroclaw-labs/zeroclaw/src/agent/prompt.rs`
- `AgentBuilder::prompt_builder(...)` in `repocache/zeroclaw-labs/zeroclaw/src/agent/agent.rs`
- Interactive loop prompt setup in `repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs`

## Provider & Model

The provider layer is rich and close to `brain`'s interests. Providers advertise capabilities such as native tool calling and vision, and the runtime can route across model/provider chains. `providers/reliable.rs` shows the real operational story: fallback models, fallback providers, retry/backoff, auth rotation, and streaming detection.

The tradeoff is centralization. Routing and reliability policy are core-runtime concerns rather than thin provider-trait wrappers.

ZeroClaw does have meaningful mode-like concepts, but they are more infrastructural than user-facing. Scenario routing, autonomy levels, native-tool vs prompt-tool provider behavior, compact-context flags, and delegate-agent configuration collectively act as the mode system. There is no evidence of a polished first-class read-only “plan mode” comparable to Codex or OpenCode in the current snapshot.

## Tool System

`ZeroClaw` has a real `Tool` trait and a concrete local coding-tool suite. File read/write/edit and glob are native Rust implementations, while content search shells out to `rg` with fallback to `grep`. The runtime can present tool schemas natively or use prompt-driven fallback parsing when the provider lacks structured tool support.

Approval and policy integrate directly with tool execution. The loop checks `ApprovalManager`, path and autonomy policy, and can refuse tools or require confirmation depending on the configured autonomy level.

## Storage & Sessions

Conversation history is mainly in-memory, but long-lived memory is durable. `brain.db` stores memories with FTS5 search and optional embeddings, while `MEMORY_SNAPSHOT.md` provides a cold-boot bootstrap artifact at workspace scope. Credentials live in `auth-profiles.json` and can be encrypted via `SecretStore`.

There is no rich persisted session model comparable to Codex or OpenCode. The workspace path is the primary durable scope, and `session_id` appears mainly in memory entries rather than in a dedicated transcript store.

## Server/Client Architecture

The repo is a single-binary design with optional long-running modes. The interactive CLI runs everything in-process. `zeroclaw gateway start` launches an Axum HTTP/WebSocket/SSE gateway on top of the same runtime, and `zeroclaw daemon` adds background services such as channel adapters, heartbeat handling, and cron tasks.

The boundary is concrete rather than trait-first: gateway handlers share `AppState`, and each WebSocket connection creates an in-process agent rather than attaching to a separately isolated session engine.

## Security & Permissions

`ZeroClaw` has a concrete, code-backed security story. Approval is handled by `ApprovalManager`, autonomy policies gate operations, path confinement and resolved-path checks are explicit, and runtime adapters can switch between native execution and stronger backends such as Docker. Public gateway binding is also blocked unless explicitly allowed.

The main caveat is that the strength of the sandbox depends heavily on runtime choice and configuration.

## CLI & TUI

`ZeroClaw` is CLI plus gateway/dashboard rather than a terminal-TUI product. The main user-facing commands are the agent CLI, `gateway start`, daemon/service flows, auth/model commands, and diagnostics. The web dashboard and API/gateway matter more than ratatui-style terminal UX. For the full side-by-side CLI matrix, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

Planning artifacts are therefore mostly emergent rather than explicit: summaries, compaction output, and delegated-agent results exist, but there is not a clearly branded planning mode with its own transcript semantics.

## Mapping to brain Traits

- `Provider`: first-class and one of the best external matches.
- `Tool`: first-class with concrete built-ins and adaptive dispatch.
- `Store`: first-class through memory and auth storage.
- `AgentLoop`: strong, but still implicit inside one large runtime module.
- `Transport`: first-class through channel and gateway abstractions.

## Key Takeaways

- `Steal:` provider capability flags, iterative loop design, reliability/failover policy, and explicit prompt-section composition.
- `Steal:` native file tools plus explicit bootstrap truncation rules.
- `Differentiate:` keep the crate graph smaller and the core loop less overloaded.
- `Differentiate:` make durable sessions cleaner and more explicit if `brain` wants transcript-first behavior.
- `Differentiate:` make sandbox integration less dependent on optional runtime selection.

## Key Evidence

- `repocache/zeroclaw-labs/zeroclaw/src/agent/loop_.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/agent/prompt.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/agent/agent.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/providers/reliable.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/tools/delegate.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/memory/snapshot.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/gateway/mod.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/gateway/ws.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/channels/cli.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/security/policy.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/runtime/native.rs`
- `repocache/zeroclaw-labs/zeroclaw/src/runtime/docker.rs`
