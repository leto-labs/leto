# async-openai

## One-Line Take

`async-openai` is best treated as an upstream OpenAI protocol and streaming reference for `brain-providers`, not as a direct peer to `brain`'s full engine architecture.

## Snapshot

- Vertical: Rust SDK for OpenAI and OpenAI-compatible APIs.
- Best comparison inside `brain`: typed request and response coverage, streaming semantics, realtime protocol support, and hosted tool schemas.
- Main lesson: this repo is extremely useful for `brain-providers/openai`, but not for `brain`'s full five-trait engine shape.

## Tool Call Method

`async-openai` models tools explicitly at the API level. It contains typed representations for function calling, hosted tools, MCP-related schemas, shell-related schemas, file search, apply-patch, code interpreter, and related OpenAI protocol surfaces.

The key limitation is that these are protocol types and client helpers, not a local coding-tool runtime. The crate describes and transports tool interactions; it does not implement local file/search/shell tools the way `brain-tools` does.

## Provider / Model Method

This is the repo's main purpose. `async-openai` provides explicit config and client abstractions for OpenAI and OpenAI-compatible endpoints, including Azure variants. Models are configured through typed request builders and client/config objects rather than through a broad provider-runtime trait system.

That makes it narrower than `brain`'s provider architecture, but highly valuable as a wire-format reference.

## Agent Loop Method

There is no reusable local agent loop as a core concept. The crate exposes primitives and examples, while orchestration is left to callers or to higher-level OpenAI product flows such as conversations and responses.

This means it is not a meaningful `AgentLoop` competitor to `brain`; it is an upstream API building block.

## Permissions / Sandbox Method

The crate serializes approval-related and environment-related protocol fields, but it does not enforce a local sandbox or permissions layer of its own. Approval and execution semantics are largely delegated to the OpenAI APIs and hosted tools being modeled.

So it is a good protocol reference, not a good local security-runtime reference.

## Platform Support

`async-openai` is a Rust SDK with some WASM support, although not every feature is available in WASM environments. It is not trying to be a product with a broad OS/runtime surface the way the coding-agent and orchestration platforms are.

## Technical Architecture

The repo is a protocol SDK workspace with:

- a main client crate
- config abstractions
- typed request and response structs
- feature-gated API groups
- examples for responses, realtime, conversations, file search, and hosted tools

This is one layer below `brain`'s full engine concerns and is best understood as raw material for `brain-providers`.

## Mapping To `brain` Core Traits

- `Provider`: first-class.
- `Tool`: first-class, but only at the protocol/schema level.
- `Store`: implicit through hosted APIs such as conversations, files, and vector stores, not as a local reusable store abstraction.
- `AgentLoop`: missing as a central abstraction.
- `Transport`: missing as an app/runtime abstraction, even though the crate supports HTTP, SSE, and realtime protocols.

This is why `async-openai` belongs in the docs as a provider reference rather than a full competitor.

## Concrete Tool Implementation Notes

- `FileRead`: not centralized as a local tool; file-related behavior is mostly upload/download and hosted retrieval.
- `FileWrite`: delegated to hosted APIs or remote tool semantics.
- `FileEdit`: delegated to hosted APIs such as apply-patch-like tools.
- `Glob` / `Find`: no generic local built-in.
- `Grep` / content search: delegated to hosted file-search APIs, not local grep.
- `Shell` / `Bash`: modeled in protocol types, but not implemented as a local shell executor.

So the repo is excellent for tool schemas and event types, but not for concrete local tool-runtime implementation.

## What To Steal

- Typed coverage of OpenAI protocol surfaces.
- SSE and realtime streaming semantics.
- Hosted-tool schema modeling and MCP-related wire types.

## What To Differentiate

- Keep `brain` strong on local engine concerns such as tool execution, durable store, and loop orchestration.
- Avoid collapsing provider support into one provider-specific SDK shape.
- Build a richer local runtime model above the protocol layer.

## Key Evidence

- `repocache/64bit/async-openai/async-openai/README.md`
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
