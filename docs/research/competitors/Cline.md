# Cline

## Overview

`Cline` is one of the strongest benchmarks in this set for an IDE-hosted coding-agent runtime: explicit plan/act modes, broad provider coverage, durable task persistence, checkpoints, MCP, browser tooling, and a shared task core reused across VS Code, standalone, and CLI shells.

| Item | Value |
| --- | --- |
| Vertical | VS Code-first TypeScript coding agent with webview UX, standalone host mode, and Ink CLI |
| Best comparison inside `brain` | Task orchestration, plan/act collaboration flow, checkpoint-backed coding tools, MCP integration, and host/runtime separation |
| Main lesson | `Cline` proves that a rich coding-agent runtime can be shared across multiple shells, but most of its seams are still product seams rather than a small reusable engine kernel |

## Architecture

`Cline` is structurally larger than a typical editor extension. The operational center is the task runtime under `src/core/task/`, where `Task` wires together provider access, context management, browser sessions, terminal execution, checkpoints, prompt building, message persistence, and tool dispatch. `Controller` and `WebviewProvider` wrap that runtime for the default VS Code/webview experience, while `StateManager` and the shared storage layer keep global state, task history, and per-task files coherent.

The important architectural fact is that `Cline` is not just a sidebar prompt wrapper. The repo also carries a standalone host path in `src/standalone/`, external host adapters in `src/hosts/external/`, and a separate CLI package in `cli/`. Those surfaces all reuse the same broad task/controller stack rather than each owning an independent agent loop.

Compared with `brain`, this is product-first architecture with a real reusable runtime core inside it. Provider, tool, storage, prompt, host, and transport concepts are all present, but they are not top-level interchangeable traits. The runtime is shared, yet still strongly coupled to Cline-specific UI state, history formats, approvals, and host abstractions.

## Agent Loop

The loop is task-driven and iterative. `Task` owns the working state for a user request, including `TaskState`, message history, context tracking, tool execution, approvals, checkpoints, and resume behavior. The runtime creates a provider-specific `ApiHandler`, builds a system prompt for the current mode, streams the model response, parses tool uses, executes the selected tool, pushes tool results back into the message history, and continues until the task is complete or the user intervenes.

Streaming is first-class. `ApiHandler.createMessage(...)` returns an `ApiStream`, and `StreamResponseHandler` turns partial provider output into reasoning blocks and tool-use blocks while the response is still arriving. Native tool calls are treated as structured streamed content rather than a single end-of-turn blob, which lets Cline surface partial tool intent and reasoning in the UI before execution is finalized.

Tool dispatch is coordinated rather than ad hoc. `ToolExecutor` builds a `TaskConfig`, registers tool handlers through `ToolExecutorCoordinator`, validates each call, and routes execution across file tools, command execution, MCP, browser actions, and plan-mode response helpers. This is a concrete subsystem seam, but it is still a Cline-owned seam rather than a reusable tool trait boundary.

Planning is embedded in the loop as an explicit collaboration state. `Mode` is literally `"plan" | "act"`, the system prompt has a dedicated plan-vs-act section, and there is a dedicated `plan_mode_respond` tool. Cline also has a separate deep-planning prompt path that guides the model to create a structured implementation handoff via `new_task`, which is stronger than a plain “think before coding” convention.

Compaction and context control are integrated into task flow. `ContextManager` tracks context history updates, inspects prior token usage, and decides when the runtime should compact or condense. Cline also carries summarize/condense handlers and a `useAutoCondense` setting, which makes context management a runtime concern rather than a caller concern.

Recovery behavior is stronger than many IDE-first competitors. Tasks can be resumed from persisted history, incomplete request artifacts are cleaned up on resume, corrupt task history can be reconstructed, and task state is protected with a mutex because multiple async subsystems can modify it. Checkpoints add another recovery layer for file-editing workflows.

Subagent support exists, but it is lighter than Codex child threads or OpenCode session forks. The prompt/tool surface includes `new_task` and `subagent`, and deep-planning explicitly hands work off into a new task context, but the architecture is still centered on one primary task runtime rather than a general persisted child-session tree.

Key types and functions:

- `Task` in `repocache/cline/cline/src/core/task/index.ts`
- `ToolExecutor` in `repocache/cline/cline/src/core/task/ToolExecutor.ts`
- `StreamResponseHandler` in `repocache/cline/cline/src/core/task/StreamResponseHandler.ts`
- `TaskState` in `repocache/cline/cline/src/core/task/TaskState.ts`
- Deep planning prompt generation in `repocache/cline/cline/src/core/prompts/commands/deep-planning/index.ts`

## System Prompt & Prompt Building

`Cline` has one of the more structured prompt systems in the comparison set. The prompt is not just a hardcoded markdown asset: `PromptRegistry` selects a model-family variant, `PromptBuilder` assembles component sections, and `ClineToolSet` exposes either prompt-described tools or native tool definitions depending on the active model/runtime path.

The prompt itself is mode-aware and model-aware. The plan/act instructions live in a dedicated component, model families pick different prompt variants, and the registry can return versioned or tagged prompt variants for specific model behaviors. This is closer to a prompt architecture subsystem than to one static system prompt string.

User instruction injection is also substantial. Cline reads `.clinerules`, recursive `AGENTS.md`, workflow and hook directories, and skills from project and global directories including `.clinerules/skills`, `.cline/skills`, `.claude/skills`, and `.agents/skills`. `RuleContextBuilder` activates rules based on current task paths, visible tabs, open tabs, and pending/completed tool operations, which is more dynamic than simple file inclusion.

Context shaping is rich but still Cline-specific. `ContextManager` tracks prior edits and truncation state, and Cline also maintains file, model, and environment context trackers that persist task metadata separately from the transcript itself. That gives the runtime more environmental awareness than a flat chat-history model.

Key types and functions:

- `PromptRegistry` in `repocache/cline/cline/src/core/prompts/system-prompt/registry/PromptRegistry.ts`
- `getSystemPrompt(...)` in `repocache/cline/cline/src/core/prompts/system-prompt/index.ts`
- Plan/act component in `repocache/cline/cline/src/core/prompts/system-prompt/components/act_vs_plan_mode.ts`
- Skills discovery in `repocache/cline/cline/src/core/context/instructions/user-instructions/skills.ts`
- Rule evaluation context in `repocache/cline/cline/src/core/context/instructions/user-instructions/RuleContextBuilder.ts`
- Response-format helpers in `repocache/cline/cline/src/core/prompts/responses.ts`

## Provider & Model

Provider support is one of Cline's broadest concrete strengths. `src/core/api/index.ts` is effectively a large provider registry that can instantiate handlers for Anthropic, OpenAI, OpenAI-compatible backends, Gemini, Bedrock, OpenRouter, Ollama, LM Studio, Claude Code, Cline-hosted modes, and many others. That is operationally strong, but it is configuration-heavy rather than trait-light.

Mode and provider selection are tightly connected. Plan and act can use different models, different reasoning-effort settings, and different provider-specific parameters. The runtime therefore treats “mode” as a concrete provider/model policy boundary, not just a UI label.

This makes Cline a strong benchmark for real provider breadth and model-family-aware behavior, but not for a minimal swappable provider interface in the `brain` sense.

## Tool System

The tool system is broad and better structured than many editor agents. Tool specifications live under the system-prompt tool registry, while execution is handled by `ToolExecutorCoordinator`, validators, and dedicated handlers. Cline supports file read/write/replace, command execution, browser actions, MCP tool use, MCP resource access, web fetch/search, follow-up questions, completion, plan responses, and task handoff.

The important detail is that Cline supports both prompt-described tools and native tool calling. `PromptRegistry` can expose native tools for the selected variant, and `StreamResponseHandler` knows how to parse streamed native tool-use deltas into Cline's internal tool-use representation.

MCP is also deeply integrated rather than bolted on. `McpHub` manages server configuration, auth, transport selection, live reload, tool listing, prompt/resource access, and notification delivery into active tasks. That makes Cline one of the stronger MCP product references in the set.

## Storage & Sessions

`Cline` persists more than many IDE-first tools. Per-task state is stored under a task directory with files such as `ui_messages.json`, `api_conversation_history.json`, `task_metadata.json`, and task-local settings. Global task listing is tracked separately in `taskHistory.json`.

This is not a clean Project/Session/Message store abstraction. It is a product storage design organized around task directories and host state. Still, it is durable and operationally mature enough to support resume, repair, checkpoints, metadata tracking, and cross-shell reuse.

The storage layer also spans multiple scopes. `ClineStorage` and sync-storage adapters abstract key/value storage, while `disk.ts` centralizes on-disk task files, settings, rules, hooks, skills, cache artifacts, and remote config cache. Workspace-local artifacts such as `.clinerules`, `AGENTS.md`, and skill directories participate directly in runtime behavior.

## Server/Client Architecture

Default `Cline` is an in-host client/runtime product, not a general remote attach server. The core experience runs through a host shell such as VS Code, with `WebviewProvider` owning the UI shell and `Controller`/`Task` owning the runtime beneath it.

At the same time, the repo clearly aims beyond one host. `standalone/cline-core.ts` boots a paired standalone service, sets up a host bridge, creates a shared storage context, and exposes a local protobus service. The CLI package then reuses the same underlying runtime with an Ink interface and external-host shims. This is a real multi-shell design, just not a Codex/OpenCode-style multi-client app server.

For `brain`, the lesson is useful: a runtime can be shared across IDE, standalone, and CLI shells without first becoming a generic network daemon. The tradeoff is that the transport boundary stays product-shaped and host-specific.

## Security & Permissions

`Cline` is approval-heavy rather than sandbox-heavy. It has strong human-in-the-loop controls, command permission handling, follow-up question flows, auto-approve toggles, and checkpoint-backed reviewability. The product surface is explicit about letting the user approve or reject steps as the task progresses.

What it does not have is a strong built-in hard sandbox comparable to Codex or the stronger ZeroClaw/IronClaw runtime options. Execution still happens through host terminals, local file access, browser automation, and external provider calls. That makes Cline a strong permission UX reference, but not the best isolation reference.

## CLI & TUI

The default UX is IDE/webview-first, and that matters architecturally. `Cline`'s main product surface is the VS Code activity-bar panel backed by `WebviewProvider`, not a terminal-first TUI in the Codex or OpenCode sense.

Still, the terminal story is no longer trivial. The separate CLI package uses React Ink, supports plan and act flags, continue/resume behavior, reasoning-effort overrides, piped stdin handling, MCP shortcuts, and a standalone host context. That makes Cline more than “just an extension”, but the CLI is still downstream of the shared product runtime rather than the defining shell.

Planning is especially relevant here. Cline exposes explicit Plan and Act modes to the user, and the CLI can select them directly. That is one of the clearest user-facing planning workflows in the comparison set, even though the surrounding UX is more IDE/webview-driven than TUI-driven.

## Mapping to brain Traits

- `Provider`: first-class in practice, but registry/config-driven rather than trait-minimal.
- `Tool`: first-class with prompt/native registration plus coordinated execution.
- `Store`: concrete and durable, but product-owned rather than a swappable storage trait.
- `AgentLoop`: real and iterative, but embedded in `Task` and related runtime modules.
- `Transport`: present through host bridges, webviews, standalone services, and CLI shells, but not exposed as a small engine transport interface.

## Key Takeaways

- `Steal:` explicit `plan` / `act` mode semantics, task resume and repair flows, checkpoint-backed coding workflows, and broad MCP integration.
- `Steal:` model-family-aware prompt variants plus dynamic instruction/skill injection.
- `Steal:` the idea of one shared runtime reused across IDE, standalone, and CLI shells.
- `Differentiate:` keep provider, store, loop, and transport as engine traits instead of product subsystems centered on one task object.
- `Differentiate:` keep the runtime less entangled with host UI state, approvals, and task-history file formats.
- `Differentiate:` pair approval UX with a stronger hard sandbox if `brain` wants safer local execution than Cline currently provides.

## Key Evidence

- `repocache/cline/cline/src/core/task/index.ts`
- `repocache/cline/cline/src/core/task/ToolExecutor.ts`
- `repocache/cline/cline/src/core/task/StreamResponseHandler.ts`
- `repocache/cline/cline/src/core/task/TaskState.ts`
- `repocache/cline/cline/src/core/prompts/system-prompt/registry/PromptRegistry.ts`
- `repocache/cline/cline/src/core/prompts/system-prompt/components/act_vs_plan_mode.ts`
- `repocache/cline/cline/src/core/prompts/commands/deep-planning/index.ts`
- `repocache/cline/cline/src/core/api/index.ts`
- `repocache/cline/cline/src/core/context/context-management/ContextManager.ts`
- `repocache/cline/cline/src/core/context/context-management/context-window-utils.ts`
- `repocache/cline/cline/src/core/context/instructions/user-instructions/skills.ts`
- `repocache/cline/cline/src/core/context/instructions/user-instructions/RuleContextBuilder.ts`
- `repocache/cline/cline/src/core/storage/disk.ts`
- `repocache/cline/cline/src/shared/storage/ClineStorage.ts`
- `repocache/cline/cline/src/services/mcp/McpHub.ts`
- `repocache/cline/cline/src/integrations/checkpoints/factory.ts`
- `repocache/cline/cline/src/core/webview/WebviewProvider.ts`
- `repocache/cline/cline/src/standalone/cline-core.ts`
- `repocache/cline/cline/cli/src/index.ts`
