# OpenCode

## Overview

`OpenCode` is a product-complete coding agent runtime with polished session UX, but its core loop is tightly fused to product concerns rather than exposed as a small reusable engine.

| Item | Value |
| --- | --- |
| Vertical | Full AI coding product spanning CLI/TUI, desktop, VS Code, HTTP server, SDK, and web surfaces |
| Best comparison inside `brain` | Session orchestration, agent profiles, streaming UX, and approval-heavy tool execution |
| Main lesson | Rich coding-agent UX comes from an integrated runtime, but that same integration makes the core less cleanly swappable |

## Architecture

`OpenCode` is a Bun/TypeScript monorepo, but the important architectural fact is that much of the runtime lives under `packages/opencode/src`. The `session/` package owns orchestration, message serialization, compaction, summaries, and streaming. `agent/agent.ts` defines built-in agent profiles such as `build`, `plan`, `general`, `explore`, `compaction`, `title`, and `summary`. `provider/` adapts model APIs, `tool/` centralizes the built-in and registry-backed tool surface, and `cli/cmd/tui/*.ts` plus `server/server.ts` provide the client/server shell around the same session runtime.

The repo is best described as a modular monolith. Provider wiring, permissions, snapshots, retry logic, MCP, task delegation, and UI streaming are all wired through the same runtime path rather than separated into independent engine traits.

## Agent Loop

The core loop is iterative, not single-pass. `session/prompt.ts` rehydrates the session history, checks for pending `subtask` or `compaction` work, detects context overflow, resolves tools, builds the system prompt, and then calls `SessionProcessor.process(...)`. After a tool-using turn, compaction event, or subtask summary, it re-enters the loop and re-queries the model instead of treating tool execution as the end of the turn.

Streaming is handled inside `session/processor.ts` by consuming `LLM.stream(streamInput).fullStream`. The processor reacts to token-level and structured events such as `text-start`, `text-delta`, `reasoning-delta`, `tool-call`, `tool-result`, `tool-error`, `start-step`, and `finish-step`, updating persisted session parts as the stream advances. `session/llm.ts` uses AI SDK `streamText(...)` and applies provider-specific prompt/message transforms through `ProviderTransform.message(...)`.

Tool dispatch is model-driven and event-fed. `session/prompt.ts` resolves the active tool set from built-ins, agent policy, config tools, plugin tools, MCP, and structured-output injection. Execution itself is delegated through the streaming call, and `SessionProcessor` consumes the resulting `tool-call` and `tool-result` events to persist tool parts and decide whether the loop should `continue`, `stop`, or `compact`. Tool results feed back through `MessageV2.toModelMessages(...)` on the next loop iteration. The repo explicitly supports multi-tool turns for explore/subtask workflows, but the provider stream is still consumed as one serialized event stream.

Artifact generation is also mode-shaped. The built-in `title`, `summary`, and `compaction` agents generate session metadata and compressed history artifacts, while `plan` mode is explicitly read-only and geared toward producing plan documents rather than mutating the workspace.

Compaction is a first-class loop concern. `session/prompt.ts` checks `SessionCompaction.isOverflow(...)` before and after turns. If overflow is detected, it creates or resumes a compaction task. `SessionProcessor` also flips to `"compact"` when a turn finishes over the token budget or when a context-overflow error is raised. `tool/truncation.ts` complements this by spilling oversized tool output to a file and telling the agent to use `Task`, `Grep`, or paged `Read` rather than stuffing the full output back into context.

Retry and recovery are built into the processor layer. `session/llm.ts` passes `maxRetries` into `streamText`, while `session/processor.ts` maps provider failures through `MessageV2.fromError(...)`, uses `SessionRetry.retryable(...)` and `SessionRetry.delay(...)`, and updates session status to `retry` before sleeping and retrying. Doom-loop protection is explicit: `SessionProcessor` compares the last three tool parts and, if the same tool call repeats with the same input three times, asks for `doom_loop` permission through `PermissionNext.ask(...)`.

Subagents and child sessions are first-class. `session/prompt.ts` recognizes pending `subtask` parts, runs the delegated work, summarizes the result back into the parent session, and contains prompt guidance telling the model to launch up to three explore agents in parallel when appropriate. Agent profiles also include a `compaction` helper agent, plus `title` and `summary` agents for session metadata generation.

Max-turn control exists as `agent.steps` in `agent/agent.ts`, enforced in `session/prompt.ts` with `const maxSteps = agent.steps ?? Infinity`. When the last step is reached, the loop injects `session/prompt/max-steps.txt` as an assistant reminder rather than allowing unlimited recursion.

Key types and functions:

- `SessionPrompt.loop(...)` in `repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts`
- `SessionProcessor.create(...).process(...)` in `repocache/anomalyco/opencode/packages/opencode/src/session/processor.ts`
- `LLM.stream(...)` in `repocache/anomalyco/opencode/packages/opencode/src/session/llm.ts`
- `SessionCompaction.create/process/isOverflow(...)` in `repocache/anomalyco/opencode/packages/opencode/src/session/compaction.ts`
- `Agent.get(...)` and built-in agent definitions in `repocache/anomalyco/opencode/packages/opencode/src/agent/agent.ts`
- `Truncate.output(...)` in `repocache/anomalyco/opencode/packages/opencode/src/tool/truncation.ts`

## System Prompt & Prompt Building

The static prompt is partly hardcoded and partly file-backed. `session/system.ts` picks model-family prompt fragments such as `prompt/codex_header.txt`, `prompt/anthropic.txt`, `prompt/beast.txt`, `prompt/gemini.txt`, and `prompt/trinity.txt`. `session/llm.ts` then either injects those fragments as system messages or, for Codex-style models, sends `SystemPrompt.instructions()` via the provider `instructions` channel instead of the normal provider system prompt path.

Dynamic prompt assembly happens in `session/prompt.ts`. Each turn adds environment metadata from `SystemPrompt.environment(model)`, an available-skills block from `SystemPrompt.skills(agent)`, and any queued instruction overlay from `InstructionPrompt.system()`. Structured-output mode appends `STRUCTURED_OUTPUT_SYSTEM_PROMPT`, and plan/build agents inject additional turn-specific guidance from prompt text files such as `plan.txt`, `build-switch.txt`, and `max-steps.txt`.

There is a prompt-template system, but it is closer to agent/profile prompt assets and command templates than to a generalized handlebars-style renderer. The repo relies on included `.txt` prompt fragments plus command-template expansion in `session/prompt.ts`. Project-specific `AGENTS.md` or rule-file injection is not evident as a first-class mechanism in the current source snapshot; the nearest analogue is the dynamic skills list and instruction overlays.

Context-window management is explicit but decentralized. `MessageV2.filterCompacted(...)` removes already-compacted history from the active transcript, `SessionCompaction.isOverflow(...)` checks token usage against model limits, and `tool/truncation.ts` prevents giant tool outputs from expanding the prompt. `session/llm.ts` also omits `maxOutputTokens` for Codex and GitHub Copilot providers while using `ProviderTransform.maxOutputTokens(...)` elsewhere.

Message history formatting is handled by `MessageV2.toModelMessages(...)` plus provider-specific repair in `provider/transform.ts`. `session/llm.ts` wraps the final model with middleware that rewrites the prompt using `ProviderTransform.message(...)`, and it can inject a dummy `_noop` tool for LiteLLM/Anthropic-proxy compatibility when history contains old tool calls but the current turn has no active tools.

Key types and functions:

- `SystemPrompt.instructions/provider/environment/skills` in `repocache/anomalyco/opencode/packages/opencode/src/session/system.ts`
- `SessionPrompt.resolvePromptParts(...)` and turn assembly in `repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts`
- `LLM.stream(...)` prompt construction in `repocache/anomalyco/opencode/packages/opencode/src/session/llm.ts`
- `ProviderTransform.message(...)` in `repocache/anomalyco/opencode/packages/opencode/src/provider/transform.ts`
- Prompt assets under `repocache/anomalyco/opencode/packages/opencode/src/session/prompt/`
- Agent prompt assets under `repocache/anomalyco/opencode/packages/opencode/src/agent/prompt/`

## Provider & Model

Provider support is catalog-driven and adapter-heavy. `provider/provider.ts` and its companion schema/transform modules merge provider metadata, auth state, config, and plugin state before creating the active model instance. This is flexible and product-ready, but it is configuration-first rather than a small language-level interface like `brain`'s `Provider` trait.

`OpenCode` also treats mode as agent profile. `build`, `plan`, `general`, and `explore` can each override model choice, prompt behavior, permissions, tool set, and generation parameters. That is powerful UX, but it couples provider selection tightly to product personas.

The mode system is more concrete than in most competitors. `agent/agent.ts` defines `build`, `plan`, `general`, `explore`, `compaction`, `title`, and `summary` as built-in agents. `plan` mode explicitly disallows edit tools and grants `plan_exit` plus plan-file write paths, while `build` mode can transition out of plan mode via `plan_enter`. This means planning is not just a UI toggle; it is a permission- and artifact-aware runtime profile.

## Tool System

`OpenCode` builds the model-facing tool surface from a central registry plus runtime filters. `session/prompt.ts` resolves built-in tools, config-defined tools, plugin tools, MCP tools, and optional structured-output helpers, then hands the resulting tool map to `session/llm.ts`. Built-ins adapt to provider/model capabilities; for example some families get `apply_patch` while others keep separate `edit` and `write`.

Execution is wrapped in policy and post-processing. Tool results are persisted as structured message parts, can be truncated to spill files, and are fed back into the next loop turn. Approval is driven through `PermissionNext`, with agent-level allow/ask/deny rules and special checks such as `doom_loop`.

## Storage & Sessions

Persistence is SQLite-backed and richer than `brain`'s minimal data model. Global data lives under the XDG data/config roots, with `opencode.db` storing projects, workspaces, sessions, messages, parts, and account state. Project-local state can also exist in `.opencode/` and `opencode.json`.

The session model is especially rich. `SessionTable` tracks project/workspace linkage, parent sessions for forks, summaries, diffs, permission state, compaction timestamps, and archive state. Messages are split into `MessageTable` and `PartTable`, which allows the UI to persist text, reasoning, tool calls, patches, snapshots, subtask records, and compaction records as separate structured items rather than one flat message blob.

## Server/Client Architecture

The default TUI is client/server even when everything stays in one process. `opencode` spawns a Worker thread that exposes the same server surface the remote client uses. `opencode serve` starts a headless HTTP server, and `opencode attach <url>` connects a remote TUI over HTTP plus SSE. `opencode run --attach <url>` uses the same remote boundary for non-interactive runs.

The API surface is Hono-based REST plus SSE, and the JS SDK is generated from the OpenAPI spec. The process model is less daemon-oriented than OpenClaw: the server usually dies with the CLI unless the user is explicitly running `serve`.

## Security & Permissions

`OpenCode` is approval-heavy rather than strongly sandboxed. The permission model is explicit and good for UX: rules are `allow`, `ask`, or `deny`, scoped by tool and pattern, and can vary by agent. The runtime also checks for protected directories and external workspace access.

The main limitation is that execution is still host execution through spawned processes. `OpenCode` is therefore a strong reference for permission UX and guardrails, but not for hard isolation boundaries.

## CLI & TUI

The CLI/TUI surface is one of `OpenCode`'s strongest differentiators. The default `opencode` command launches the TUI, `opencode serve` runs the headless server, `opencode attach <url>` connects to a remote server, `opencode run` supports non-interactive runs, and session commands support resume, fork, export, MCP, and account flows. The TUI includes a header/footer layout, session picker, sidebar, slash commands, inline tool rendering, and a leader-key-oriented keybinding system. For the full cross-competitor comparison, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

Mode switching is part of the UX rather than a buried config concept. The TUI and CLI expose agent selection, and the default visible primary modes are effectively the operational personas of the product: build when the model should act, plan when it should stay read-only and produce a plan artifact, and explore/general for research-oriented work.

## Mapping to brain Traits

- `Provider`: first-class in practice, but configuration-led.
- `Tool`: first-class via the central registry and typed tool parts.
- `Store`: first-class in implementation, but not as a small swappable trait.
- `AgentLoop`: implicit runtime boundary, not a standalone engine trait.
- `Transport`: implicit runtime boundary through Worker RPC, HTTP, SSE, and the SDK.

## Key Takeaways

- `Steal:` session-driven orchestration, explicit compaction handling, agent profiles that package tools plus model plus permissions, and strong streaming/tool UX.
- `Steal:` remote `serve` plus `attach` as a product-grade transport pattern.
- `Differentiate:` keep the engine seams explicit instead of burying them in one runtime package.
- `Differentiate:` prefer a stronger execution boundary than approval rules alone.
- `Differentiate:` avoid coupling the core engine to desktop, server, SDK, and TUI concerns.

## Key Evidence

- `repocache/anomalyco/opencode/packages/opencode/src/agent/agent.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/processor.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/llm.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/system.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/compaction.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/provider/provider.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/provider/transform.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/registry.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/truncation.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/thread.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/worker.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/attach.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/serve.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/run.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/server/server.ts`
