# Codex

## Overview

`Codex` is the strongest Rust product benchmark in this set: excellent sandboxing, polished tooling, and durable session infrastructure, but with most of the runtime concentrated in `codex-core`.

| Item | Value |
| --- | --- |
| Vertical | Full Rust AI coding product spanning CLI/TUI, app server, MCP server, remote clients, and IDE integration |
| Best comparison inside `brain` | Tool routing, sandboxing, session persistence, multi-agent support, and app-server transport boundaries |
| Main lesson | `Codex` validates many of `brain`'s design instincts, but it solves them inside one concrete product runtime rather than a small trait kernel |

## Architecture

The workspace is broad, but `codex-rs/core` is the center of gravity. `codex.rs` owns session initialization, turn orchestration, prompt building, compaction triggers, event emission, and history persistence. `tools/` contains the strongest clean seam in the repo via `ToolHandler`, `ToolRegistry`, and `ToolRouter`. `context_manager/` and `state/session.rs` own history normalization and token accounting. `compact.rs` and `compact_remote.rs` handle summarization, while `agent/control.rs` adds subagent spawning and thread forking. The app-server, TUI, connectors, and sandbox crates all wrap or feed back into this same core runtime.

This is more modular than a monolith, but it is still product-first architecture. Provider, store, loop, and transport are concrete implementations that happen to be well-factored, not interchangeable top-level traits.

## Agent Loop

The loop is iterative and turn-driven. `CodexThread` exposes `submit(...)`, `steer_input(...)`, and `next_event(...)`, but the real work happens in `codex.rs`: build the prompt, send a sampling request, stream `ResponseEvent`s, dispatch tools, append tool results to history, and continue until there is no follow-up work left. This is not single-pass; tool calls and pending user steering can extend a turn.

Streaming is first-class. `run_sampling_request(...)` constructs a `Prompt`, creates a `ToolCallRuntime`, and then consumes `ResponseEvent` values such as `OutputItemAdded`, `OutputItemDone`, `OutputTextDelta`, `ReasoningSummaryDelta`, `RateLimits`, and `Completed`. The event stream updates UI/front-end consumers through emitted protocol events while simultaneously recording the same items into rollout/history state.

Tool dispatch is routed, typed, and partially parallelized. `build_prompt(...)` advertises `parallel_tool_calls` when the selected model supports them. `ToolRouter` maps model-visible specs to handlers, and `handle_output_item_done(...)` creates tool futures that are queued into `in_flight`. Mutating tools are gated by `tool_call_gate`, and tool outputs are fed back into the next prompt through the session history as `ResponseInputItem`/`ResponseItem` values.

Codex also has an explicit planning path inside the loop. The streaming code in `codex.rs` maintains `plan_mode_state` and handles assistant item emission differently while plan mode is active, which is stronger than a simple “don’t run tools” instruction. Planning is treated as a dedicated collaboration mode that changes how turn output is shaped before the user approves further action.

Compaction is deeply integrated with loop control. `codex.rs` checks total token usage against each model's `auto_compact_token_limit`, can run pre-sampling compaction when switching to a smaller context window, and triggers either inline or remote summarization. `compact.rs` builds a summarization prompt from `templates/compact/prompt.md`, retries streaming if the compaction request disconnects, trims older history if the summary prompt itself overflows, and replaces history with a compacted transcript plus reinjected initial context.

Retry and error recovery are robust. Sampling retries respect `turn_context.provider.stream_max_retries()`. When the retry budget is exhausted, the runtime can switch fallback transport from WebSocket to HTTPS before giving up. Retry notifications are surfaced to the UI as stream errors, `ResponseEvent::RateLimits` snapshots update session state, and context-window overflow is escalated explicitly as `CodexErr::ContextWindowExceeded`.

Subagents are first-class, not bolted on. `agent/control.rs` provides `spawn_agent(...)` and `spawn_agent_with_options(...)`, can fork rollout history into a child session, and records the child thread as `SessionSource::SubAgent(...)`. The main tool surface also exposes `spawn_agent`, `send_input`, `resume_agent`, `wait`, and `close_agent`.

The runtime has multiple turn/loop guards. Tool execution is bounded by the model and prompt design rather than one simple `max_iterations` field, but compaction thresholds, transport retry budgets, plan-mode state, and history token limits all act as loop controls. Child runs inherit session source and can fork history rather than recursively mutating the same thread forever.

Key types and functions:

- `CodexThread::{submit, steer_input, next_event}` in `repocache/openai/codex/codex-rs/core/src/codex_thread.rs`
- `run_sampling_request(...)` and `build_prompt(...)` in `repocache/openai/codex/codex-rs/core/src/codex.rs`
- `ContextManager` in `repocache/openai/codex/codex-rs/core/src/context_manager/history.rs`
- `run_inline_auto_compact_task(...)` in `repocache/openai/codex/codex-rs/core/src/compact.rs`
- `spawn_agent_with_options(...)` in `repocache/openai/codex/codex-rs/core/src/agent/control.rs`
- `ToolRouter` in `repocache/openai/codex/codex-rs/core/src/tools/router.rs`

## System Prompt & Prompt Building

The static base prompt is hardcoded as markdown assets. `core/gpt_5_1_prompt.md` is the canonical built-in system/developer prompt, and it already bakes in important harness expectations such as AGENTS.md handling, update-plan behavior, patch usage, and validation style. Compaction uses its own static prompt assets from `templates/compact/prompt.md` and `templates/compact/summary_prefix.md`.

Dynamic prompt assembly happens in `codex.rs`. Session initialization loads `user_instructions` via `get_user_instructions(&config)`, merges base instructions from config or conversation history, resolves dynamic tools, and stores the result in the turn context. `build_prompt(...)` then serializes the current history plus tool specs into a `Prompt { input, tools, parallel_tool_calls, base_instructions, personality, output_schema }`.

`AGENTS.md` injection is explicit and source-backed. `instructions/user_instructions.rs` wraps AGENTS content using the prefix `# AGENTS.md instructions for ...` and serializes it into a structured response item fragment. Skills are handled similarly through `SkillInstructions`. Codex also supports user prompt templates via `custom_prompts.rs`, which discovers markdown files under `$CODEX_HOME/prompts`, parses frontmatter, and surfaces them as slash-command-style custom prompts.

Context-window management is one of Codex's strongest areas. `ContextManager` normalizes history, tracks model-visible bytes, estimates token usage, and prepares the exact history slice sent to the model. `state/session.rs` stores that manager, updates token info after each response, and supports replacement with compacted history. When the prompt exceeds the model window, Codex can compact, drop older items during compaction itself, or error explicitly if the turn cannot be made to fit.

Message history formatting is concrete and structured. History is not sent as a flat chat transcript string; it is stored as `ResponseItem`/`RolloutItem` data and normalized by `ContextManager` before building the next `Prompt`. That gives Codex strong invariants around tool-call/result ordering, multimodal items, compaction items, and persisted replay state.

Key types and functions:

- `gpt_5_1_prompt.md` in `repocache/openai/codex/codex-rs/core/gpt_5_1_prompt.md`
- `build_prompt(...)` in `repocache/openai/codex/codex-rs/core/src/codex.rs`
- `UserInstructions` in `repocache/openai/codex/codex-rs/core/src/instructions/user_instructions.rs`
- `discover_prompts_in(...)` in `repocache/openai/codex/codex-rs/core/src/custom_prompts.rs`
- `ContextManager` in `repocache/openai/codex/codex-rs/core/src/context_manager/history.rs`
- `Session` state in `repocache/openai/codex/codex-rs/core/src/state/session.rs`

## Provider & Model

Provider support is registry-driven rather than trait-driven. `ModelProviderInfo` carries built-in and configured providers such as OpenAI, Ollama, and LM Studio, while `ModelClient` is the concrete runtime that speaks the Responses API. Model metadata includes tool-support flags like `shell_type`, `apply_patch_tool_type`, and `web_search_tool_type`, and the models manager refreshes server model data when it receives new `ModelsEtag` events.

Modes are spread across config, sandbox policy, approval mode, and plan/review flows. Codex does not expose one unified `Mode` trait, but it does let profiles and tools shape the runtime in similarly powerful ways.

The mode system is broad enough to matter as its own design pattern. There is an explicit `/plan` command in the TUI, a `review` command path, model/reasoning-effort selection, fast mode, collaboration modes, remote mode, and config profiles. In practice, Codex treats “mode” as a bundle of UI state, provider/model policy, and loop behavior rather than one enum.

## Tool System

Tools are the cleanest subsystem seam in the repo. `ToolHandler` defines the handler contract, `ToolRegistryBuilder` registers concrete built-ins and MCP tools, and `ToolRouter` mediates between model-visible specs and execution. Payloads are strongly typed, and Codex distinguishes function tools, MCP tools, local shell calls, tool search, and custom-tool flows.

The built-in tool surface is broad: file read/list/search, shell/PTTY execution, `apply_patch`, MCP resource access, image viewing, plan updates, permissions requests, and subagent control. Mutating tools integrate tightly with approvals and sandboxing via the `tool_call_gate`.

## Storage & Sessions

Codex persists threads as both JSONL rollouts and SQLite metadata. JSONL is the source-of-truth event log under `~/.codex/sessions/rollout-*.jsonl`, while `state_5.sqlite` mirrors thread metadata, dynamic tools, memories, and agent jobs for listing and search. Archived sessions move to `archived_sessions/`.

The message/session model is event-rich rather than chat-flat. `RolloutItem` includes session metadata, response items, compaction records, turn-context metadata, and event messages. Threads store `cwd` and git metadata directly instead of introducing a separate project entity. Credentials can be stored in `auth.json`, OS keyring, or age-encrypted secrets.

## Server/Client Architecture

Codex uses a hybrid in-process and out-of-process app-server model. The default TUI still talks to an app-server boundary, but `InProcessAppServerClient` implements that boundary with channels inside the same process. `codex app-server` exposes the same protocol over stdio or WebSocket, and `codex --remote ws://...` swaps in a remote client without changing the TUI architecture.

This is an important design validation for `brain`: the UI is always a client of an API boundary, even when the "server" lives in the same process.

## Security & Permissions

Codex has the strongest built-in sandboxing in the comparison set. `SandboxPolicy` encodes read-only, workspace-write, danger-full-access, and external-sandbox modes, with platform-specific implementations for macOS Seatbelt, Linux Landlock/seccomp, and Windows restricted tokens. Approval is layered on top through explicit permission tools and gating for mutating operations.

This is the best benchmark in the set for serious local execution controls.

## CLI & TUI

The CLI/TUI surface is broad and unusually mature. `codex` launches the default interactive UI, `codex exec` handles non-interactive runs, `codex app-server` serves IDE and remote clients, and the TUI exposes slash commands like `/model`, `/compact`, `/review`, `/fork`, `/permissions`, `/init`, `/ps`, and `/stop`. The app-server also enables remote TUI attach over WebSocket. For the full command and keybinding comparison, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

For planning specifically, Codex is the clearest explicit benchmark in the set. `/plan` switches the TUI into Plan mode, and the app tracks planning state deeply enough to affect event emission and reasoning-scope prompts instead of treating planning as just another prompt convention.

## Mapping to brain Traits

- `Provider`: implicit, implemented as provider registries plus concrete `ModelClient`.
- `Tool`: first-class via `ToolHandler`, `ToolRegistry`, and `ToolRouter`.
- `Store`: implicit concrete storage via JSONL plus SQLite, not a store trait.
- `AgentLoop`: implicit inside `codex.rs` and `Session`.
- `Transport`: implicit via app-server, TUI, exec, MCP server, and remote WebSocket clients.

## Key Takeaways

- `Steal:` typed tool routing, JSONL-plus-SQLite session persistence, serious sandboxing, compaction flow, and the app-server boundary.
- `Steal:` multi-agent spawning with thread forking rather than ad hoc child prompts.
- `Differentiate:` keep provider, store, loop, and transport as explicit engine traits instead of concrete `codex-core` subsystems.
- `Differentiate:` keep the crate graph smaller and the central runtime less dominant.
- `Differentiate:` use the Codex lessons without inheriting its product-level complexity.

## Key Evidence

- `repocache/openai/codex/codex-rs/core/src/codex.rs`
- `repocache/openai/codex/codex-rs/core/src/codex_thread.rs`
- `repocache/openai/codex/codex-rs/core/src/compact.rs`
- `repocache/openai/codex/codex-rs/core/src/compact_remote.rs`
- `repocache/openai/codex/codex-rs/core/src/context_manager/history.rs`
- `repocache/openai/codex/codex-rs/core/src/state/session.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/router.rs`
- `repocache/openai/codex/codex-rs/core/src/tools/registry.rs`
- `repocache/openai/codex/codex-rs/core/src/agent/control.rs`
- `repocache/openai/codex/codex-rs/core/src/custom_prompts.rs`
- `repocache/openai/codex/codex-rs/core/src/instructions/user_instructions.rs`
- `repocache/openai/codex/codex-rs/core/gpt_5_1_prompt.md`
- `repocache/openai/codex/codex-rs/app-server/src/in_process.rs`
- `repocache/openai/codex/codex-rs/tui/src/lib.rs`
- `repocache/openai/codex/codex-rs/tui_app_server/src/lib.rs`
