# Rig

## One-Line Take

`Rig` is a strong Rust SDK and library benchmark for provider, tool, and RAG composition, but it is not a full coding-agent runtime or orchestration platform.

## Snapshot

- Vertical: Rust LLM and agent application SDK.
- Best comparison inside `brain`: provider abstractions, tool plumbing, MCP integration, and vector-store composition.
- Main lesson: `Rig` shows how far a Rust library-first design can go without becoming a full product runtime.

## Tool Call Method

`Rig` has first-class tool abstractions. It defines tool traits, tool sets, tool servers, and MCP-oriented adapters, which makes it one of the cleaner library comparisons for `brain`'s `Tool` trait.

What it does not try to do is ship a built-in coding-agent tool suite. The framework centralizes tool plumbing and tool composition, while concrete app tools are expected to be authored by the application or provided through integrations.

## Provider / Model Method

This is one of `Rig`'s strongest areas. It has explicit provider/client abstractions, capability descriptions, multiple provider modules, and strong vector-store and retrieval composition around that provider layer.

Relative to `brain`, `Rig` is a very useful design reference for library-grade provider APIs, but it is more retrieval and application oriented than engine-kernel oriented.

## Agent Loop Method

`Rig` does have a real multi-turn agent flow with max-turn control, hooks, and tool continuation. The important nuance is that this loop is implemented as part of the concrete agent machinery rather than exposed as a top-level pluggable `AgentLoop` trait.

That makes it a useful comparison for loop behavior, but less of a perfect structural match than `brain` or `ZeroClaw`.

## Permissions / Sandbox Method

The safety model is hook-based rather than sandbox-based. Hook callbacks can approve, skip, or terminate tool calls, but the repo does not define a centralized OS sandbox, path policy, or shell isolation layer comparable to a hardened coding-agent runtime.

So `Rig` is stronger on SDK composition than on runtime safety boundaries.

## Platform Support

`Rig` is Rust-native and library-first. It includes integrations such as CLI chatbot and Discord bot support, and there is some WASM history, but the runtime story is less productized than the coding-agent platforms in this comparison set.

## Technical Architecture

The repo is a modular Rust workspace centered on `rig-core`, with companion crates and integrations layered around it. This makes it more library-shaped than the large product monorepos and closer in spirit to `brain`, even though the actual trait breakdown is different.

The strongest architectural emphasis is on:

- provider abstractions
- tool abstractions
- vector stores and retrieval
- reusable application SDK building blocks

## Mapping To `brain` Core Traits

- `Provider`: first-class.
- `Tool`: first-class.
- `Store`: implicit; strong vector-store support exists, but not a general session/message store abstraction like `brain`.
- `AgentLoop`: implicit; real loop behavior exists, but not as a top-level pluggable trait.
- `Transport`: implicit; integrations exist, but not a repo-wide engine transport boundary.

This makes `Rig` a strong secondary benchmark for `brain`, especially below the full runtime/product layer.

## Concrete Tool Implementation Notes

- `FileRead`: not centralized as a built-in coding-agent tool; typically delegated to app-authored tools.
- `FileWrite`: same as file read.
- `FileEdit`: same as file write.
- `Glob` / `Find`: not centralized as an agent tool, though ingestion loaders support file and glob-based loading.
- `Grep` / content search: not a local coding-agent grep tool; retrieval is typically vector-store or provider-hosted search oriented.
- `Shell` / `Bash`: not provided as a built-in shell runtime.

So while `Rig` has excellent tool abstractions, it is a weak comparison point for the concrete coding-tool layer.

## What To Steal

- Clean library-facing provider and tool abstractions.
- MCP and tool-server composition.
- Retrieval and vector-store integration patterns.

## What To Differentiate

- Keep `brain` stronger on durable store, transport, and engine-loop boundaries.
- Provide a clearer local coding-tool story than library-first frameworks usually do.
- Build stronger runtime policy and sandbox boundaries where `Rig` stays hook-oriented.

## Key Evidence

- `repocache/0xPlaygrounds/rig/README.md`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/lib.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/client/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/providers/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/tool/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/tool/server.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/tools/think.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/vector_store/mod.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/agent/completion.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/agent/prompt_request/hooks.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/src/loaders/file.rs`
- `repocache/0xPlaygrounds/rig/rig/rig-core/tests/permission_control.rs`
