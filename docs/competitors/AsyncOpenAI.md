# async-openai

## Overview

`async-openai` is best treated as an upstream OpenAI protocol and streaming reference for `brain-providers`, not as a direct peer to `brain`'s full engine architecture.

| Item | Value |
| --- | --- |
| Vertical | Rust SDK for OpenAI and OpenAI-compatible APIs |
| Best comparison inside `brain` | Typed request/response coverage, SSE streaming semantics, hosted-tool schemas, and realtime protocol support |
| Main lesson | Extremely useful for `brain-providers/openai`, but not a meaningful benchmark for `brain`'s full five-trait engine shape |

## Architecture

This repo is a protocol SDK workspace, not an agent runtime. The main crate provides client/config abstractions, typed request/response structs, feature-gated API groups, and examples for Responses, Realtime, Conversations, file search, MCP-related types, and hosted tools.

It is one layer below `brain`'s full engine concerns and is best understood as raw material for provider implementation.

## Agent Loop

Not a core concern for this project. `async-openai` exposes primitives and examples, but it does not implement a reusable local agent loop. Higher-level orchestration is left to callers or remote OpenAI product flows.

## System Prompt & Prompt Building

Not a core concern for this project. The crate transports prompts and conversation items, but it does not define a local prompt-builder system, AGENTS injection pipeline, or context-management policy for an agent runtime.

## Provider & Model

This is the repo's main purpose. `async-openai` provides explicit config and client abstractions for OpenAI and OpenAI-compatible endpoints, including Azure variants. Models are configured through typed request builders and config objects rather than through a broad multi-provider runtime trait system.

## Tool System

`async-openai` models tools explicitly at the protocol level. It includes typed representations for function calling, hosted tools, MCP-related schemas, shell/container surfaces, file search, apply-patch-style tools, and related OpenAI wire formats.

The key limitation is that these are schemas and client helpers, not a local coding-tool runtime.

## Storage & Sessions

N/A as a local engine concern. The repo models hosted APIs such as conversations and files, but it does not ship a local session/message store abstraction comparable to `brain`.

## Server/Client Architecture

N/A as an app/runtime architecture. The crate speaks HTTP, SSE, and realtime protocols, but does not define an end-user CLI/TUI or daemon boundary.

## Security & Permissions

The crate serializes approval-related and execution-related protocol fields, but it does not enforce a local sandbox or permission model. Approval semantics are delegated to the hosted APIs being modeled.

## CLI & TUI

N/A. `async-openai` is a library SDK rather than a terminal product.

## Mapping to brain Traits

- `Provider`: first-class.
- `Tool`: first-class, but only at the protocol/schema level.
- `Store`: implicit through hosted APIs, not a local store abstraction.
- `AgentLoop`: missing as a central concern.
- `Transport`: missing as an app/runtime abstraction.

## Key Takeaways

- `Steal:` typed OpenAI protocol coverage, SSE/realtime semantics, and hosted-tool schema modeling.
- `Differentiate:` keep `brain` strong on local engine concerns such as tool execution, durable store, and loop orchestration above the protocol layer.

## Key Evidence

- `repocache/64bit/async-openai/async-openai/src/config.rs`
- `repocache/64bit/async-openai/async-openai/src/client.rs`
- `repocache/64bit/async-openai/async-openai/src/responses/conversations.rs`
- `repocache/64bit/async-openai/async-openai/src/types/responses/response.rs`
- `repocache/64bit/async-openai/async-openai/src/types/responses/stream.rs`
- `repocache/64bit/async-openai/async-openai/src/types/mcp/mcp_.rs`
- `repocache/64bit/async-openai/async-openai/src/types/containers/container.rs`
- `repocache/64bit/async-openai/examples/responses-function-call/src/main.rs`
- `repocache/64bit/async-openai/examples/assistants-file-search/src/main.rs`
- `repocache/64bit/async-openai/examples/assistants-code-interpreter/src/main.rs`
- `repocache/64bit/async-openai/examples/realtime/src/main.rs`
