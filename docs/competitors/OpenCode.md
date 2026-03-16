# OpenCode

## One-Line Take

`OpenCode` is a product-complete AI coding platform with a large integrated runtime, not a minimal reusable engine.

## Snapshot

- Vertical: end-user coding agent product spanning CLI, desktop, IDE, HTTP server, SDK, and web surfaces.
- Best comparison inside `brain`: product pressure on top of `AgentLoop`, `Transport`, and permission orchestration.
- Main lesson: rich product behavior comes from a deeply integrated runtime, but that same integration makes the core less cleanly composable.

## Tool Call Method

`OpenCode` builds tools from a central runtime registry. The model-facing tool surface can include built-in tools, config-defined JS/TS tools, plugin tools, and MCP tools. The set is adaptive: some models get `apply_patch`, others get `edit` and `write`, and some search tools are gated behind provider-specific capabilities.

Execution is heavily productized. Tool runs pass through plugin hooks, schema validation, result truncation, and context-management logic before their outputs are fed back into the session. This is much closer to a product shell around tool use than to `brain`'s smaller `Tool` trait boundary.

## Provider / Model / Mode Method

Provider support is catalog-driven and adapter-heavy. `OpenCode` merges provider metadata, auth state, config, and plugin state, then instantiates providers through its runtime and bundled adapters. The abstraction is flexible, but it is configuration-first rather than a small language-level interface like `brain`'s `Provider` trait.

`OpenCode` also treats "mode" as "agent". Built-in agents such as `build`, `plan`, `general`, and `explore` can each override model, prompt, permissions, tools, and generation settings. That makes agent behavior a first-class product construct instead of a narrow loop primitive.

## Agent Loop Method

The loop is persistent and session-centric. It rehydrates history, creates a new assistant turn, streams model output, records structured session parts, executes tools, handles retries, and can trigger compaction or child-session delegation. It also includes doom-loop protection when the same tool call repeats too often.

This is operationally mature and user-friendly, but it is tightly coupled to product concerns such as session snapshots, UI streaming, compaction workers, and child task sessions. `brain` can stay cleaner by keeping those concerns outside the smallest orchestration core.

## Permissions / Sandbox Method

The safety model is primarily policy-based: `allow`, `ask`, and `deny` rules can be scoped by tool or wildcard pattern and overridden per agent. File access can detect external directories and protect certain OS paths, but command execution is still host execution through spawned processes.

The key implication is that `OpenCode` has a strong approval system but not a strong built-in sandbox boundary. That is useful as a UX reference, but less compelling as a hard-isolation architecture.

## Platform Support

`OpenCode` has unusually broad first-party surface area:

- terminal UI
- desktop applications
- VS Code integration
- HTTP/OpenAPI server
- JS SDK
- web client

This is one of the clearest cases where a higher-level product repo will naturally outperform a focused engine repo on user-facing breadth.

## Technical Architecture

The repo is a monorepo, but the important architectural fact is that a large amount of core behavior lives inside one dominant runtime package. Provider wiring, sessions, tools, permissions, MCP, server behavior, and project management all live close together.

That makes `OpenCode` highly capable and extensible through plugins and config, but less aligned with `brain`'s goal of a cleanly swappable engine core. It is best described as a modular monolith rather than a small composable substrate.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary via dedicated provider modules and runtime model resolution.
- `Tool`: first-class boundary via a common tool contract and central registry.
- `Store`: first-class in practice, but implemented as concrete storage modules instead of a small swappable trait.
- `AgentLoop`: implicit runtime boundary; the loop is central, but it is not exposed as a clean standalone interface.
- `Transport`: implicit runtime boundary; HTTP server, SDK, and TUI clients exist, but not behind one `Transport` abstraction.

The short version is that `OpenCode` is strongest where `brain` cares about `Provider` and `Tool`, but it is more product-runtime oriented than trait-oriented for `Store`, `AgentLoop`, and `Transport`.

## Concrete Tool Implementation Notes

- `FileRead`: implemented locally with native Node/Bun file APIs. No shell-out.
- `FileWrite`: implemented locally with native file writes. No shell-out.
- `FileEdit`: implemented locally with native fs plus diff-oriented helper logic. `apply_patch` is a separate structured tool for some model families.
- `Glob`: implemented as a ripgrep-backed file listing flow rather than a pure in-process glob implementation.
- `Grep`: explicitly shells out to `rg` and post-processes results in TypeScript.
- `Shell` / `Bash`: implemented by spawning the configured shell and layering permission analysis on top.

This makes `OpenCode` a good example of a product that keeps file mutation local but treats search primitives as wrappers around battle-tested external CLI tools.

## What To Steal

- First-class session streaming and structured event persistence.
- Strong agent-profile concept where prompts, permissions, tools, and model choice move together.
- Good operational features around compaction, retry, and child-task delegation.

## What To Differentiate

- Keep provider, tool, store, loop, and transport boundaries explicit instead of burying them in one product runtime.
- Prefer a harder execution boundary than approval rules alone.
- Avoid coupling the engine core to desktop, TUI, SDK, and server concerns.

## Key Evidence

- `repocache/anomalyco/opencode/README.md`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/agents.mdx`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/permissions.mdx`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/server.mdx`
- `repocache/anomalyco/opencode/packages/opencode/src/agent/agent.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/processor.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/provider/provider.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/registry.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/read.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/write.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/edit.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/apply_patch.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/glob.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/grep.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/file/ripgrep.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/bash.ts`
