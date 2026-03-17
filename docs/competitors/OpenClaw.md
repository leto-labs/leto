# OpenClaw

## Overview

`OpenClaw` is broader than a coding agent: it is a gateway-centric assistant platform with strong operational controls, a custom system prompt, and an embedded pi runtime for the actual agent loop.

| Item | Value |
| --- | --- |
| Vertical | Multi-surface assistant platform spanning gateway, channels, nodes, voice, tools, and remote clients |
| Best comparison inside `brain` | Gateway architecture, session serialization, delegated execution, and policy-heavy prompt/runtime integration |
| Main lesson | `OpenClaw` is excellent for control-plane ideas, but much of its loop behavior is inherited through the embedded pi runtime rather than expressed as a small local engine |

## Architecture

The repo is best understood as a gateway-centered modular monolith. The Gateway owns channel connections, agent request routing, policy, pairing, and client attachment over WebSocket. Agent execution is delegated into an embedded pi-based runtime, with OpenClaw-specific code layering custom prompt construction, session metadata, queueing, and tool policy on top.

That split matters: OpenClaw owns the outer control plane and the system prompt, while parts of the inner reasoning/tool loop come from the embedded pi packages. Any architecture read should distinguish those two layers.

## Agent Loop

OpenClaw's agent loop is explicitly documented as a serialized per-session run. `docs/concepts/agent-loop.md` describes the lifecycle as intake, session resolution, skills snapshot loading, prompt assembly, embedded pi run, stream bridging, and lifecycle completion. The main entry points are the Gateway RPC methods `agent` and `agent.wait`, plus the CLI `agent` command.

Loop serialization is one of its strongest operational ideas. `runEmbeddedPiAgent` is scheduled through per-session and optional global lanes so that only one authoritative run mutates a session at a time. Queue modes such as collect, steer, and follow-up feed into this lane system so message ordering and session history stay coherent.

Streaming is bridged rather than authored from scratch. `subscribeEmbeddedPiSession` translates embedded pi session events into OpenClaw stream channels: `assistant` deltas, `tool` events, and `lifecycle` events for `start`, `end`, and `error`. `agent.wait` then blocks on that lifecycle stream instead of polling the session transcript.

Tool dispatch is a combination of embedded pi execution and OpenClaw-specific policy. The embedded runtime handles the core model/tool iteration, while OpenClaw adds tool-result sanitization, tool-policy enforcement, reply suppression for messaging-tool duplicates, and compaction-related retry/reset behavior. Some tools can also be delegated to connected clients, making tool execution transport-aware in a way most repos do not attempt.

Compaction is visible at the gateway layer. The concept docs state that auto-compaction emits stream events and can trigger a retry, resetting in-memory buffers and tool summaries to avoid duplicate output. The actual compaction mechanics inherit heavily from the embedded pi runtime, but OpenClaw clearly treats compaction as part of the session lifecycle rather than an invisible background detail.

Retry and error recovery include timeouts and lifecycle fallbacks. `runEmbeddedPiAgent` enforces an abort timeout, `agent.wait` has its own wait timeout, and `agentCommand` emits lifecycle end/error events if the embedded loop fails to do so. That makes the outer loop more operationally reliable than many pure in-process CLIs.

Subagents exist but are intentionally lighter. The system-prompt docs note a `minimal` prompt mode for sub-agents and reduced bootstrap injection for them. This shows that OpenClaw does support child agent contexts, but the repo positions them as smaller embedded runs rather than separate persisted first-class thread objects.

OpenClaw also generates execution-planning artifacts in a narrower operational sense. The `system.run` approval flow builds explicit approval plans, hardens them, and then treats those plans as authoritative command context at execution time. That is not “plan mode” in the Codex sense, but it is a concrete example of agent-generated artifacts controlling later execution.

Key types and functions:

- `docs/concepts/agent-loop.md`
- `buildAgentMessageFromConversationEntries(...)` in `repocache/openclaw/openclaw/src/gateway/agent-prompt.ts`
- Gateway request flow in `repocache/openclaw/openclaw/src/gateway/server-chat.ts`
- Session and lane handling under `repocache/openclaw/openclaw/src/sessions/`
- Embedded runner under `repocache/openclaw/openclaw/src/agents/pi-embedded-runner/`

## System Prompt & Prompt Building

OpenClaw explicitly owns its system prompt. `docs/concepts/system-prompt.md` states that the runtime does not use the default pi-coding-agent system prompt. Instead, OpenClaw builds a custom prompt with fixed sections for Tooling, Safety, Skills, Self-Update, Workspace, Documentation, Workspace Files, Sandbox, Current Date & Time, Reply Tags, Heartbeats, Runtime, and Reasoning.

Dynamic prompt assembly is one of the most explicit in the comparison set. The prompt can render in `full`, `minimal`, or `none` modes. Workspace bootstrap injection appends `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, `BOOTSTRAP.md`, and `MEMORY.md` when available, with per-file and total character caps. Subagents only inject `AGENTS.md` and `TOOLS.md` to keep the context small.

OpenClaw also exposes hook points around prompt assembly. The docs call out gateway hooks such as `agent:bootstrap` and plugin hooks like `before_model_resolve` and `before_prompt_build`, which can prepend context, replace the system prompt, or append system context before submission.

Context-window management is treated as an operator concern, not just an implementation detail. The docs explicitly warn that bootstrap files consume context on every turn and can increase compaction frequency. `buildAgentMessageFromConversationEntries(...)` also shows another optimization choice: the gateway collapses conversation history into a single structured string message for the embedded runtime, preferring the latest user/tool entry as the current message and formatting earlier entries as history context.

There is no evidence that OpenClaw uses a standalone markdown prompt-template system like pi-mono's prompt templates for the core agent prompt. It relies on custom prompt building plus hook-based overrides.

Key types and functions:

- `docs/concepts/system-prompt.md`
- `buildAgentMessageFromConversationEntries(...)` in `repocache/openclaw/openclaw/src/gateway/agent-prompt.ts`
- Prompt build hooks described in `docs/concepts/agent-loop.md`
- Gateway/agent integration under `repocache/openclaw/openclaw/src/gateway/`

## Provider & Model

Model selection is config-first. OpenClaw normalizes models around `provider/model` identifiers, supports defaults and per-agent overrides, and adds fallback and auth-profile policy at the gateway layer. It is practical product architecture, but much more registry- and config-driven than trait-driven.

The important nuance is that some model/runtime behavior is inherited from the embedded pi runtime, while OpenClaw-specific code controls outer policy, routing, and prompt shape.

The mode system is richer than it first appears. Besides `promptMode` (`full`, `minimal`, `none`), sessions track queue modes, thinking and verbose levels, reasoning visibility, elevated execution levels, and runtime choices such as ACP vs subagent execution. OpenClaw is a good example of a platform where “mode” is spread across prompt rendering, runtime routing, and operator policy rather than one chat-only toggle.

## Tool System

Tools are first-class and policy-rich. OpenClaw advertises typed tool schemas to the model, filters them by agent/profile/session policy, and supports host-executed tools, sandboxed execution, plugin tools, and delegated client tools. That last category is distinctive: a connected client can receive a pending tool operation and execute it outside the gateway process.

The visible product tool surface is broader than the set of tools clearly implemented locally in this repo, because some coding-tool behavior is inherited from the embedded pi runtime.

## Storage & Sessions

OpenClaw has a very rich persisted session model. Sessions carry agent ID, run metadata, token counters, labels, channel origin, queue mode, sandbox state, and other operational fields, with transcript JSONL files stored per agent. The agent, not the project, is the main organizational unit.

Config and secrets are also layered: `openclaw.json` plus env-based secret providers and auth profiles control provider access and agent defaults.

## Server/Client Architecture

OpenClaw is the clearest daemon-first architecture in the set. `openclaw gateway` is the central long-lived process, and CLI clients, macOS app, web UI, and device nodes all attach over WebSocket. The Gateway owns the channels and remains alive after CLI clients disconnect.

This makes OpenClaw less like the usual in-process coding-agent CLI and more like a persistent control plane.

## Security & Permissions

Security is one of OpenClaw's strongest areas. The system combines gateway auth and pairing, tool policy, Docker sandboxing, execution approvals, and controlled elevated execution. The main caveat is that in-process plugins remain outside the sandbox boundary, which weakens the extension trust model compared with the core runtime.

## CLI & TUI

The CLI is a client of the gateway rather than the main runtime host. Commands cover gateway lifecycle, setup/configuration, health, model inspection, sessions, sandbox helpers, plugins, and TUI/client attachment. OpenClaw also inherits `pi-tui`-style terminal UX for the client side, but the gateway and multi-surface attachment model are the more important differentiators. For the full command and keybinding comparison, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

## Mapping to brain Traits

- `Provider`: first-class in practice through provider/auth registries.
- `Tool`: first-class via typed tools and policy enforcement.
- `Store`: implicit and distributed through sessions, memory, and agent config.
- `AgentLoop`: implicit, split between gateway orchestration and embedded pi runtime.
- `Transport`: first-class and arguably the repo's strongest subsystem.

## Key Takeaways

- `Steal:` gateway queueing/serialization, client attachment model, delegated tool execution, and explicit prompt ownership.
- `Steal:` operator-facing documentation for loop and prompt behavior.
- `Differentiate:` keep the engine core independent of a daemon-centric gateway.
- `Differentiate:` keep extension boundaries safer than in-process unsandboxed plugins.
- `Differentiate:` preserve cleaner trait seams for loop and store than OpenClaw currently exposes.

## Key Evidence

- `repocache/openclaw/openclaw/docs/concepts/agent-loop.md`
- `repocache/openclaw/openclaw/docs/concepts/system-prompt.md`
- `repocache/openclaw/openclaw/src/gateway/agent-prompt.ts`
- `repocache/openclaw/openclaw/src/gateway/server-chat.ts`
- `repocache/openclaw/openclaw/src/gateway/server-methods.ts`
- `repocache/openclaw/openclaw/src/gateway/server-lanes.ts`
- `repocache/openclaw/openclaw/src/sessions/`
- `repocache/openclaw/openclaw/src/agents/pi-embedded-runner/`
- `repocache/openclaw/openclaw/docs/gateway/protocol.md`
- `repocache/openclaw/openclaw/docs/tools/slash-commands.md`
- `repocache/openclaw/openclaw/docs/tools/subagents.md`
