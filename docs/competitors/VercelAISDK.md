# Vercel AI SDK

## Overview

`Vercel AI SDK` is the strongest provider and tool-call API benchmark in this set, but it covers a narrower vertical than a full agent runtime.

| Item | Value |
| --- | --- |
| Vertical | Application and agent SDK for AI apps, not a durable standalone agent platform |
| Best comparison inside `brain` | Provider design, streaming semantics, tool-call protocol, and UI-facing transport ideas |
| Main lesson | It is extremely polished where `brain` needs provider and tool API quality, but it does not try to own the entire engine stack |

## Architecture

The repo is a highly composable TypeScript monorepo. Package boundaries are clear and meaningful: provider interfaces, provider utilities, generation APIs, tool protocols, framework adapters, MCP support, and UI transport layers are all separated well.

This is composable SDK architecture, not a small durable runtime kernel.

## Agent Loop

There is an agent loop, but it is deliberately focused. Helpers such as `ToolLoopAgent`, `generateText()`, and `streamText()` orchestrate text generation plus tool execution with stop conditions. This is practical and useful, but much thinner than a persistent session runtime with its own store and transport abstractions.

## System Prompt & Prompt Building

Prompt construction is API-centric rather than repo-centric. The SDK gives callers ergonomic ways to pass prompts, messages, tools, and output schemas, but it does not define a canonical system-prompt builder, AGENTS-style file injection pipeline, or durable prompt-template system for a coding-agent runtime.

## Provider & Model

This is the repo's strongest area. `ProviderV4` and `LanguageModelV4` standardize model factories and generation interfaces across providers and modalities. Model selection is ergonomic in both direct-provider and gateway-style `provider/model` forms.

## Tool System

Tool calling is first-class. The SDK exposes `tool()` and `dynamicTool()`, and its generation flows handle tool-call parsing, validation, execution, and continuation. It also models provider-executed tools and approval collection for deferred tool execution.

## Storage & Sessions

Store is implicit only. Chat state exists in examples and UI helpers, but durable session/message storage is not a repo-wide first-class runtime concern.

## Server/Client Architecture

The repo provides UI-facing transport helpers and framework integrations, but not one canonical CLI/TUI/server product architecture. It is designed to be embedded into applications.

## Security & Permissions

Approval is modeled as a first-class protocol concern, which is stronger than many SDKs. Sandboxing, however, is external. The repo does not define a core local execution isolation layer comparable to the hardened coding-agent products.

## CLI & TUI

N/A as a core product surface. Vercel AI SDK is primarily an application SDK for web and Node environments rather than a terminal-first product.

## Mapping to brain Traits

- `Provider`: first-class and one of the strongest in the set.
- `Tool`: first-class through tool types, approvals, and execution helpers.
- `Store`: implicit only.
- `AgentLoop`: first-class, but focused on text-and-tool iteration.
- `Transport`: first-class for UI/chat protocol helpers.

## Key Takeaways

- `Steal:` provider/model interface quality, tool-call ergonomics, and polished streaming semantics.
- `Differentiate:` keep store, loop, and transport as first-class engine concepts instead of stopping at SDK helpers.

## Key Evidence

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
