# Vercel AI SDK

## One-Line Take

`Vercel AI SDK` is the strongest provider and tool-call API benchmark in this set, but it covers a narrower vertical than a full agent runtime.

## Snapshot

- Vertical: application and agent SDK for AI apps, not a full durable agent platform.
- Best comparison inside `brain`: `Provider` design, streaming semantics, tool-call protocol, and UI-facing transport ideas.
- Main lesson: the SDK is extremely polished where `brain` needs provider abstraction, but it does not try to own the entire engine stack.

## Tool Call Method

Tool calling is a first-class API. The SDK exposes `tool()` and `dynamicTool()`, and its `generateText()` and `streamText()` flows handle tool-call parsing, validation, execution, and continuation. It also supports provider-executed tools and approval flows.

This is a very strong reference for the wire protocol and developer ergonomics of tool use. The key distinction from `brain` is that the SDK provides orchestration helpers around model interaction rather than a full standalone tool runtime with store and transport abstractions.

## Provider / Model / Mode Method

This is the repo's strongest area. `ProviderV4` and `LanguageModelV4` standardize model factories and generation interfaces across providers and modalities. The package structure is clean and composable, with shared provider utilities and individual provider packages layered on top.

Model selection is ergonomic in two styles: direct provider calls and gateway-style `provider/model` references. This is a strong benchmark for how `brain-providers` should feel from an API design perspective, even if the host language and product scope differ.

## Agent Loop Method

There is an agent loop, but it is deliberately focused. `ToolLoopAgent` is basically a helper around text generation plus tool execution with a stop condition. It is useful and practical, but it is not a full persistent runtime kernel with its own store, session model, or transport abstraction.

That means `Vercel AI SDK` pressures `brain` less on architecture breadth and more on the quality of its tool-loop API.

## Permissions / Sandbox Method

Approval is modeled as a first-class protocol concern, including provider-side tool approvals and approval collection for deferred tool execution. That is more structured than many ad hoc agent repos.

Sandboxing, however, is external. The SDK does not define a core engine isolation layer comparable to a host runtime or sandbox abstraction. If `brain` wants a strong permissions story, it should treat `Vercel AI SDK` as a protocol reference rather than a sandbox reference.

## Platform Support

The SDK is extremely broad inside the JavaScript ecosystem:

- Node.js
- Next.js
- React
- Vue
- Svelte
- Angular
- LangChain and LangGraph integrations

This is much more of an app-builder platform than a terminal or daemon product.

## Technical Architecture

This is a highly composable monorepo rather than a monolith. Package boundaries are clear and meaningful: provider interfaces, provider utilities, orchestration APIs, framework adapters, MCP support, and UI transport layers are all separated well.

The important limit is that `Store`, `Transport`, and `AgentLoop` are not first-class engine traits in the way `brain` wants them to be. The repo is composable, but it is composable as an SDK family, not as a small general-purpose agent kernel.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary and one of the strongest in the set.
- `Tool`: first-class boundary via tool types, preparation, approvals, and execution hooks.
- `Store`: implicit only; chat state exists, but not as a repo-wide durable store abstraction.
- `AgentLoop`: first-class boundary, but focused on text-and-tool iteration rather than a full persistent runtime kernel.
- `Transport`: first-class boundary for UI chat transport and MCP transport, not for a general engine transport layer.

`Vercel AI SDK` matches `brain` best on `Provider`, `Tool`, and transport protocol quality, but is much thinner on durable runtime concerns.

## Concrete Tool Implementation Notes

- `FileRead`: not a generic local built-in; closest equivalents are provider-defined editor tools.
- `FileWrite`: not a generic local built-in; implemented as provider-defined remote tool schemas where available.
- `FileEdit`: same as write; schema and adapter code exist, but execution is provider-side.
- `Glob` / `Find`: not provided as a generic local built-in.
- `Grep`: not provided as a local code-search tool; file-search tools are retrieval-oriented provider tools instead.
- `Shell` / `Bash`: present as provider-defined tool adapters, not as a local shell runtime implemented by the repo itself.

This makes `Vercel AI SDK` a strong reference for tool protocol design, but a weak reference for local filesystem and CLI tool implementation.

## What To Steal

- Provider and model interface design quality.
- Clean tool-call APIs and continuation semantics.
- Streaming protocol polish and UI-facing transport helpers.

## What To Differentiate

- Keep store, loop, and transport as first-class engine concepts.
- Own the runtime boundary instead of stopping at SDK helpers.
- Build a stronger policy and sandbox story than approval protocol alone.

## Key Evidence

- `repocache/vercel/ai/packages/ai/README.md`
- `repocache/vercel/ai/packages/ai/src/index.ts`
- `repocache/vercel/ai/packages/ai/src/agent/tool-loop-agent.ts`
- `repocache/vercel/ai/packages/ai/src/generate-text/generate-text.ts`
- `repocache/vercel/ai/packages/ai/src/generate-text/stream-text.ts`
- `repocache/vercel/ai/packages/provider/src/provider/v4/provider-v4.ts`
- `repocache/vercel/ai/packages/provider/src/language-model/v4/language-model-v4.ts`
- `repocache/vercel/ai/packages/openai/src/tool/mcp.ts`
- `repocache/vercel/ai/packages/openai/src/tool/shell.ts`
- `repocache/vercel/ai/packages/openai/src/tool/local-shell.ts`
- `repocache/vercel/ai/packages/openai/src/tool/apply-patch.ts`
- `repocache/vercel/ai/packages/anthropic/src/tool/text-editor_20250429.ts`
- `repocache/vercel/ai/packages/langchain/README.md`
