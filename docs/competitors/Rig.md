# Rig

## Overview

`Rig` is a strong Rust SDK/library benchmark for provider, tool, and RAG composition, but it is not a full coding-agent runtime or orchestration platform.

| Item | Value |
| --- | --- |
| Vertical | Rust LLM and agent application SDK |
| Best comparison inside `brain` | Provider abstractions, tool plumbing, MCP integration, and vector-store composition |
| Main lesson | `Rig` shows how far a library-first Rust design can go without becoming a full product runtime |

## Architecture

`Rig` is a modular Rust workspace centered on `rig-core`, with provider clients, tool plumbing, vector stores, loaders, and integration crates layered around it. It is more library-shaped than the product monorepos in this comparison set and therefore closer in spirit to `brain`, even if the trait split is different.

## Agent Loop

`Rig` does have a real multi-turn agent flow with max-turn control, hooks, and tool continuation, but it is implemented as part of the concrete agent machinery rather than as a top-level pluggable `AgentLoop` trait. That makes it a useful behavior benchmark, but not a perfect structural match.

## System Prompt & Prompt Building

Prompt construction is part of the concrete agent/request layer rather than a repo-wide prompt framework. Hooks and prompt request APIs exist, but there is no strong evidence of a generalized system-prompt builder, AGENTS-style project-context injection, or durable prompt-template system in the way the coding-agent products provide.

## Provider & Model

This is one of Rig's strongest areas. It has explicit provider/client abstractions, provider modules, and strong composition with vector stores and retrieval. Relative to `brain`, it is a very useful design reference for library-grade provider APIs.

## Tool System

`Rig` has first-class tool abstractions. It defines tool traits, tool sets, tool servers, and MCP adapters. What it does not do is ship a built-in coding-agent local tool suite. Applications are expected to provide their own concrete tools.

## Storage & Sessions

Rig has strong vector-store and retrieval support, but not a general-purpose local session/message store abstraction comparable to `brain`. Durable transcript/session storage is not a central repo concern.

## Server/Client Architecture

N/A as a central repo concern. Integrations exist, but the repo is library-first rather than centered on one CLI/TUI/daemon architecture.

## Security & Permissions

Safety is hook-based rather than sandbox-based. Hooks can approve, skip, or terminate tool calls, but the repo does not define a centralized OS sandbox or path policy layer comparable to a hardened coding-agent runtime.

## CLI & TUI

Rig has integrations and example applications, but no defining CLI/TUI product surface in the same sense as Codex, OpenCode, or pi-mono.

## Mapping to brain Traits

- `Provider`: first-class.
- `Tool`: first-class.
- `Store`: implicit through vector stores, not a general session store.
- `AgentLoop`: implicit inside concrete agent machinery.
- `Transport`: implicit through integrations, not a repo-wide transport trait.

## Key Takeaways

- `Steal:` clean library-facing provider and tool abstractions, MCP integration, and retrieval composition.
- `Differentiate:` keep `brain` stronger on durable store, transport, and engine-loop boundaries, plus local coding-tool execution.

## Key Evidence

- `repocache/0xPlaygrounds/rig/rig/rig-core/src/client/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/providers/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/tool/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/tool/server.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/vector_store/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/agent/completion.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/agent/prompt_request/hooks.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/loaders/file.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/tests/permission_control.rs`
