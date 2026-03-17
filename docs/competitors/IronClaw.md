# IronClaw

## Overview

`IronClaw` is most useful as a security-policy reference. Its architecture vocabulary is broad, but the loop and transport seams are less convincingly wired than the stronger competitors.

| Item | Value |
| --- | --- |
| Vertical | Security-first AI agent product with broad provider/channel ambitions |
| Best comparison inside `brain` | Approval, RBAC, sandbox, DLP, and policy-layer vocabulary |
| Main lesson | Strong policy design only matters if the execution path obviously runs through it |

## Architecture

`IronClaw` is a single-crate Rust application with modules for core engine/config, providers, gateway, channels, memory, RBAC, sandbox, and agents. The vocabulary is broad and ambitious, but many seams are thinner in practice than in naming. `core/engine.rs` is still the place where the main interactive flow lives, while `gateway/`, `channels/`, and parts of `agents/` feel more aspirational or partially wired.

This makes IronClaw better as a policy/reference repo than as a clean engine blueprint.

## Agent Loop

The primary loop in `core/engine.rs` is relatively simple and mostly single-pass per user turn. `run_interactive()` seeds the session with `self.config.agent.system_prompt`, reads stdin, calls the provider with the full conversation and tool schemas, executes any returned tool calls through `execute_tool_call(...)`, appends the results, prints the assistant response, increments `turn_count`, and autosaves memory. It does not obviously re-query the model multiple times inside the same turn after each tool execution the way Codex, OpenCode, ZeroClaw, or pi-mono do.

Streaming is only lightly abstracted. The `Provider` trait in `providers/mod.rs` exposes `stream_chat(...)`, but the default implementation falls back to `chat(...)`, and the main interactive flow is built around standard request/response rather than token-by-token orchestration. The separate gateway module exposes SSE and WebSocket routes, but the main CLI runtime does not appear to use that streaming path.

Tool dispatch is provider-native in concept: the provider receives tool schemas and can return tool calls, which the engine then runs through the security pipeline. The actual execution path is strongly security-focused, with RBAC, guardian checks, anti-stealer checks, SSRF protections, approval, sandboxing, DLP, cost tracking, and audit logging all inside `execute_tool_call(...)`.

Compaction and summarization are more configured than operationally central. `core/config.rs` exposes compaction thresholds and history settings, and `memory/mod.rs` includes compaction-style APIs, but prompt-history compaction is not a clear, central part of the main interactive loop in the current source snapshot.

Retry and error recovery are comparatively thin at the loop level. Providers may implement their own request behavior, but the main engine path does not show the same explicit multi-stage stream retry, transport fallback, or compaction-triggered retry logic that appears in Codex or ZeroClaw.

Subagents do exist conceptually through `agents/mod.rs`, which can orchestrate multiple named agents and aggregate results, but that looks more like a separate orchestration feature than a first-class child-session mechanism embedded into the main interactive loop.

Loop limits are explicit. `agent.max_turns` lives in config and is copied into the session as `max_turns`, with `turn_count` tracking progress. This is one of the clearer pieces of loop governance in the repo.

Planning artifacts are not a major first-class UX in the current snapshot. The repo can coordinate multiple agents and has memory compaction, but there is no clearly implemented read-only planning mode or plan-artifact workflow comparable to Codex or OpenCode.

Key types and functions:

- `Engine::run_interactive(...)` and `Engine::process_message(...)` in `repocache/JoasASantos/ironclaw/src/core/engine.rs`
- `AgentConfig` in `repocache/JoasASantos/ironclaw/src/core/config.rs`
- `Provider` trait in `repocache/JoasASantos/ironclaw/src/providers/mod.rs`
- Multi-agent support in `repocache/JoasASantos/ironclaw/src/agents/mod.rs`

## System Prompt & Prompt Building

The system prompt is mostly hardcoded through config rather than a composable builder. `core/config.rs` defines `agent.system_prompt` and `agent.max_turns`, and `core/engine.rs` seeds new interactive sessions with that configured system message before any user input.

Dynamic prompt assembly is limited in the current source snapshot. Tool schemas are passed separately to the provider, and the engine appends conversation history as `Message` structs, but there is no strong evidence of AGENTS-style project-file injection, a prompt-section builder, or a custom prompt-template system comparable to ZeroClaw, OpenClaw, Codex, or pi-mono.

Context-window management is likewise only partly evident. History/memory config exists, and memory persistence can compress or prune older data, but the main prompt-submission path does not show a clearly integrated token counter plus truncation/compaction pipeline for the live conversation.

Message history formatting is straightforward: the engine holds a `Vec<Message>` with roles, content, tool calls, tool results, and multimodal blocks, then passes that to the provider. That is simple and understandable, but much less sophisticated than the prompt/history shaping in the stronger competitors.

Key types and functions:

- `AgentConfig` in `repocache/JoasASantos/ironclaw/src/core/config.rs`
- `Message` and session flow in `repocache/JoasASantos/ironclaw/src/core/engine.rs`
- Provider request types in `repocache/JoasASantos/ironclaw/src/providers/mod.rs`

## Provider & Model

IronClaw does have a real `Provider` trait and a broad provider factory with preset aliases such as `fast`, `smart`, `cheap`, and `local`. Conceptually this is close to `brain`'s provider interests. The issue is not the abstraction direction; it is that some branches look incomplete enough to weaken the repo as an implementation-quality benchmark.

Its strongest mode concept is therefore preset selection rather than collaborative planning. Provider presets, sandbox-level presets, and CLI run vs UI modes are the main “mode system” in the repo today.

## Tool System

The intended tool model is provider-native function calling with JSON-schema-advertised tools plus a high-friction security pipeline. That design direction is sound. The current repo snapshot is less convincing about the completeness of the concrete built-in tool suite than about the surrounding policy scaffolding.

What is very clear is that any tool execution is meant to pass through layered policy checks before reaching the underlying runtime.

## Storage & Sessions

Sessions are lightweight and mostly in-memory. A session tracks `id`, `turn_count`, and `max_turns`, and conversations are stored as message vectors in process. Memory and cost tracking are durable, with SQLite-like backends and encrypted memory-store support, but transcript/session persistence is much thinner than in Codex or OpenCode.

Credentials are also lighter-weight than ZeroClaw or Codex: provider API keys live mainly in config or env vars rather than in a rich multi-profile credential vault.

## Server/Client Architecture

The main runtime is single-process and stdin-driven. `ironclaw run` reads from stdin and prints to stdout. An Axum-based web UI can be started alongside it, but the current code suggests that the engine still reads only stdin, so the web UI is not a fully authoritative client.

The gateway module exposes REST, SSE, and WebSocket routes on paper, but it is not obviously wired into the main CLI path. The channel abstraction is similarly broader in API surface than in demonstrated runtime integration.

## Security & Permissions

Security is the strongest part of the repo. IronClaw models RBAC, filesystem and network policies, approval gates, anti-stealer logic, SSRF protections, DLP, audit logging, and sandbox backends for Docker/Bubblewrap/native execution. Even where the surrounding runtime is incomplete, the policy vocabulary is rich and worth studying.

## CLI & TUI

IronClaw is CLI plus partial web UI, not a polished terminal-TUI product. The notable commands are onboarding, run/ui flows, doctor-style diagnostics, and model/config tooling. The comparison value is in its CLI and security surfaces, not in a keyboard-driven TUI. For the broader command comparison, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

## Mapping to brain Traits

- `Provider`: first-class at the type level.
- `Tool`: first-class in concept, but thinner in concrete evidence.
- `Store`: first-class through memory/storage modules.
- `AgentLoop`: implicit and comparatively simple in the main engine.
- `Transport`: first-class in naming, but only partially wired in practice.

## Key Takeaways

- `Steal:` rich policy vocabulary, config invariants, and defense-in-depth thinking.
- `Steal:` the idea that approvals, DLP, and audit should sit near tool execution rather than as UI-only affordances.
- `Differentiate:` ensure the loop actually re-queries and compacts cleanly instead of stopping at one pass.
- `Differentiate:` keep the client/server surfaces obviously wired into the real engine path.
- `Differentiate:` prefer smaller, testable subsystem boundaries over one broad monolith.

## Key Evidence

- `repocache/JoasASantos/ironclaw/src/core/engine.rs`
- `repocache/JoasASantos/ironclaw/src/core/config.rs`
- `repocache/JoasASantos/ironclaw/src/providers/mod.rs`
- `repocache/JoasASantos/ironclaw/src/gateway/mod.rs`
- `repocache/JoasASantos/ironclaw/src/memory/mod.rs`
- `repocache/JoasASantos/ironclaw/src/agents/mod.rs`
- `repocache/JoasASantos/ironclaw/src/rbac/mod.rs`
- `repocache/JoasASantos/ironclaw/src/sandbox/mod.rs`
- `repocache/JoasASantos/ironclaw/src/channels/mod.rs`
