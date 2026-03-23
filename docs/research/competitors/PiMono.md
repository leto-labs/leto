# pi-mono

## Overview

`pi-mono` is the closest TypeScript runtime analogue to `brain`: a layered stack from provider package to agent loop to coding-agent product shell.

| Item | Value |
| --- | --- |
| Vertical | TypeScript agent toolkit family with reusable runtime packages, coding-agent CLI, TUI, RPC mode, and web integrations |
| Best comparison inside `brain` | Package layering around provider APIs, session management, compaction, and multi-surface transport modes |
| Main lesson | `pi-mono` shows how to keep a meaningful runtime core even when a full coding-agent product sits on top of it |

## Architecture

The repo has clean package layering. `packages/ai` owns provider and transport-facing abstractions, `packages/agent` owns the generic loop and message model, and `packages/coding-agent` adds sessions, compaction, prompt construction, slash commands, prompt templates, skills, and CLI/TUI modes. That makes it more composable than the product-heavy monoliths in the set, even if its seams are package contracts rather than top-level traits.

The important pattern for `brain` is the progression from reusable loop core to coding-agent shell without collapsing both into one module.

## Agent Loop

`packages/agent/src/agent-loop.ts` implements an explicit iterative loop. `runLoop(...)` has an outer loop for queued follow-up messages and an inner loop for tool-call execution plus steering interruptions. It converts `AgentMessage[]` to provider-facing `Message[]` only at the LLM boundary via `convertToLlm`, then re-enters the inner loop whenever tool results or steering messages require another model turn.

Streaming is model/provider specific, but the loop is event-oriented. The loop delegates actual streaming to the configured provider stream function and surfaces tool execution, steering, and follow-up state through the agent session/event model. Unlike OpenCode or Codex, pi-mono does not turn every low-level token delta into a persisted structured session part; the core loop stays lighter.

Tool dispatch is configurable and can be parallel. `agent.ts` defaults `toolExecution` to `"parallel"`, and `agent-loop.ts` has separate paths for parallel and sequential execution while preserving ordered result insertion back into the message stream. Steering can interrupt after tool execution, and follow-up messages are only delivered once there are no more tool calls or steering messages pending.

Compaction lives one layer up in the coding-agent package. `coding-agent/src/core/compaction/compaction.ts` provides pure functions such as `shouldCompact(...)`, `serializeConversation(...)`, and `compact(...)`, while the session manager handles persistence and reload after compaction. The compaction logic chooses cut points, preserves recent context, extracts file operations, and can generate multiple summary components in parallel before merging them.

Retry and recovery are split across layers. In the generic agent layer, retry delay and maximum retry wait are configurable. In the OpenAI Codex Responses adapter, `openai-codex-responses.ts` retries transient network and rate-limit failures and surfaces friendlier error text for final failures. This is more modular than OpenCode's all-in-one runtime, but also more distributed.

Subagents are not a first-class persisted child-session system in `packages/agent`, but the coding agent supports branching and session forking. Session trees, `/fork`, branch summaries, and parent-session references are all evidence that the product shell treats branching as a first-class concept even if it is not modeled as Codex-style spawned agent threads.

Planning artifacts are present, but not as one named plan mode. Branch summaries, compaction summaries, prompt templates, and slash-command-driven workflows make pi-mono good at producing structured intermediate artifacts, yet the current source snapshot does not show a dedicated read-only planning runtime mode comparable to Codex or OpenCode.

Loop limits are explicit but softer than ZeroClaw's single constant. The generic loop stops when there are no more tool calls, steering messages, or follow-up messages, while higher-level session logic and compaction settings control context growth. Branching and compaction prevent long sessions from becoming one unbounded flat transcript.

Key types and functions:

- `runLoop(...)` in `repocache/badlogic/pi-mono/packages/agent/src/agent-loop.ts`
- `Agent` state/config in `repocache/badlogic/pi-mono/packages/agent/src/agent.ts`
- `shouldCompact(...)`, `serializeConversation(...)`, and `compact(...)` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/compaction/compaction.ts`
- OpenAI Codex Responses provider in `repocache/badlogic/pi-mono/packages/ai/src/providers/openai-codex-responses.ts`

## System Prompt & Prompt Building

The coding-agent shell builds its system prompt in code, not as one static file. `coding-agent/src/core/system-prompt.ts` exposes `buildSystemPrompt(...)`, which either wraps a custom system prompt or builds a default prompt that includes tool descriptions, guideline bullets, documentation pointers, project context files, skills, current date, and current working directory.

Dynamic prompt assembly is strong and explicit. `agent-session.ts` rebuilds the base system prompt whenever the active tool set changes, pulling in tool-specific prompt snippets and guidelines from extensions, context files from the resource loader, and preloaded skills. `resource-loader.ts` collects system prompt overrides, append-only prompt fragments, ancestor/project context files, and skills from global plus project-local locations.

`AGENTS.md`-style project context injection is present through the resource loader rather than a hardcoded AGENTS-only path. Context files are appended under `# Project Context`, and skills are injected in a compact `<available_skills>` block. The prompt is also intentionally documentation-aware for the `pi` ecosystem, pointing the model at local docs when users ask about pi-specific functionality.

There is a generalized prompt-template system for user commands. `prompt-templates.ts` loads markdown prompt files from global, project, and explicit paths, parses frontmatter, supports argument substitution like `$1`, `$@`, and `$ARGUMENTS`, and exposes those templates as slash commands in the interactive shell.

Context-window management is handled mainly through compaction settings rather than ad hoc truncation. `CompactionSettings` reserve a token budget and preserve recent tokens, while `serializeConversation(...)` and cut-point logic decide what gets summarized. The current message history is converted to LLM messages only once per turn, keeping serialization logic centralized.

Key types and functions:

- `buildSystemPrompt(...)` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/system-prompt.ts`
- `_rebuildSystemPrompt(...)` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/agent-session.ts`
- `ResourceLoader` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/resource-loader.ts`
- `loadPromptTemplates(...)` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/prompt-templates.ts`
- `formatSkillsForPrompt(...)` in `repocache/badlogic/pi-mono/packages/coding-agent/src/core/skills.ts`

## Provider & Model

The provider layer is one of pi-mono's strongest areas. `packages/ai` standardizes model/provider interaction, while the coding agent adds a model registry, provider overrides, and OAuth-aware auth handling. The OpenAI Codex Responses adapter is a good example of how the product shell still reuses the lower-level provider contract cleanly.

This is a strong reference for `brain-providers`: keep providers reusable, then let the product shell own model-selection policy and auth ergonomics.

The mode story is clearer at the product shell level than at the model layer. pi-mono has interactive, print, and RPC execution modes; thinking-level controls; session branching; and slash-command workflows that effectively change how the user collaborates with the agent. It is a useful example of a runtime where “mode” mostly lives in transports and session UX rather than in one monolithic agent profile enum.

## Tool System

Tools are layered rather than monolithic. The generic agent package works with serializable tool schemas and tool execution strategies, while `packages/coding-agent` provides the concrete coding tools: read, write, edit, grep, find, bash, and extension-defined tools. Search-heavy primitives still shell out to tools like `rg` and `fd`, while file mutations stay local and native to Node.

The system also supports extension-provided tool snippets and prompt guidelines, which feed back into system prompt construction.

## Storage & Sessions

pi-mono persists sessions as JSONL under `~/.pi/agent/sessions/<encoded-cwd>/`, with a session header followed by entry records. Session entries form a tree via parent IDs, which supports forking, branch summaries, thinking-level changes, labels, and compaction entries. Credentials live in `auth.json`, keyed by provider, and the project-local `.pi/` directory holds config and additional resources.

The data model is close to `brain` in spirit: messages remain structurally simple, but the session file contains richer event metadata than a pure flat transcript.

## Server/Client Architecture

The default modes are single-process. `pi` runs the TUI in-process, and `pi --mode print` is a one-shot text mode. The only real client/server boundary is `pi --mode rpc`, which exposes the agent over stdin/stdout JSONL and is used by IDE or external clients that spawn the agent as a subprocess.

There is no HTTP/SSE/WebSocket server. That makes pi-mono a useful transport reference for subprocess RPC rather than for long-running daemon architecture.

## Security & Permissions

Permissions are mostly app-layer and hook-based. The coding agent can ask before risky actions, and related packages such as `pi-mom` document stronger sandbox options, but there is no one engine-wide sandbox boundary comparable to Codex or even ZeroClaw's runtime adapters.

This is a place where `brain` can differentiate by making execution policy uniform across transports.

## CLI & TUI

pi-mono has a strong terminal surface. The default `pi` command launches the TUI, `pi --mode rpc` exposes the JSONL RPC mode, prompt templates and skills surface as slash commands, and the interactive shell supports commands like `/model`, `/compact`, `/fork`, `/export`, `/share`, `/tree`, and `/settings`. Its TUI also stands out for direct editing affordances and session-tree UX. For the full command and keybinding comparison, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

If `brain` wants explicit planning UX, pi-mono is the contrast case: it offers the building blocks for plan-like artifacts and branching, but not one dominant “Plan mode” concept.

## Mapping to brain Traits

- `Provider`: first-class through `packages/ai`.
- `Tool`: first-class through agent/coding-agent tool contracts.
- `Store`: first-class in practice through concrete session managers and files.
- `AgentLoop`: first-class via `packages/agent`.
- `Transport`: first-class via interactive, print, and RPC modes.

## Key Takeaways

- `Steal:` the layered progression from provider package to loop package to product shell.
- `Steal:` prompt-template loading, skill injection, compaction as a pure module, and RPC mode.
- `Differentiate:` make the engine seams explicit traits instead of package conventions.
- `Differentiate:` make permissions and sandbox behavior more uniform across surfaces.
- `Differentiate:` keep the runtime generic even when shipping a coding-agent shell on top.

## Key Evidence

- `repocache/badlogic/pi-mono/packages/agent/src/agent-loop.ts`
- `repocache/badlogic/pi-mono/packages/agent/src/agent.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/system-prompt.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/agent-session.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/resource-loader.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/prompt-templates.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/compaction/compaction.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/skills.ts`
- `repocache/badlogic/pi-mono/packages/ai/src/providers/openai-codex-responses.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/docs/rpc.md`
- `repocache/badlogic/pi-mono/packages/coding-agent/docs/tui.md`
- `repocache/badlogic/pi-mono/packages/coding-agent/docs/keybindings.md`
