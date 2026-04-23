# Project: leto

## Description
Platform-agnostic AI agent engine in Rust. The current application-facing stack
uses `agent-core` over `agent-runtime`, `agent-store`, `agent-tool`, the
`agent-tool-*` family crates, `agent-loops`, and the standalone `provider-*`
crates. Legacy `brain-*` source is archived locally under `archive/brain/` and
is not part of the committed workspace.

Repository layout keeps the product-facing runtime crates in the root
workspace and vendors reusable crate families through pinned submodules:

- `crates/agent-tool/` contains `agent-tool` plus the standalone `agent-tool-*` crates
- `submodules/atif-rust` contains the standalone `atif` workspace
- `submodules/ai-provider-rs` contains `provider` plus the standalone `provider-*` crates
- `submodules/chat-rs` contains `chat` plus the standalone `chat-*` crates

## Architecture
- Active workspace crates:
  - `agent-runtime`
  - `agent-loops`
  - `agent-tool`
  - `agent-tool-files`
  - `agent-tool-process`
  - `agent-tool-web`
  - `agent-store`
  - `agent-core`
  - `agent-core-remote`
  - `agent-server`
  - `agent-acp`
  - `agent-cli`
- Vendored submodule crates:
  - `atif`
  - `chat`
  - `chat-slack`
  - `chat-teams`
  - `chat-telegram`
  - `provider`
  - `provider-openai`
  - `provider-anthropic`
  - `provider-mistralrs`
  - `provider-llamacpp`
- Vendored submodule tool:
  - `submodules/ai-provider-rs/tools/provider-preset-gen`
- Local-only legacy archive:
  - `archive/brain/crates/brain-*`
  - archived legacy-only examples, scripts, and Harbor helpers under
    `archive/brain/`
- The modern stack splits responsibilities as:
  - `provider` and `provider-*` for the shared provider SDK and concrete integrations
  - `agent-runtime` for the live session engine
  - `agent-tool` for the shared tool SDK and runtime-facing executor contract
  - `agent-tool-files` / `agent-tool-process` / `agent-tool-web` for first-party tool families
  - `agent-store` for durable state
  - `agent-loops` for loop policy
  - `agent-core` / `agent-core-remote` / `agent-server` / `agent-acp` / `agent-cli` for application surfaces

## Language & Toolchain
- Rust, edition 2024, resolver 2
- Targets: native (Linux, macOS, Windows) and wasm32-unknown-unknown (future)

## Conventions
- Traits as interfaces, not class hierarchies
- Five core traits: Provider, Tool, Store, AgentLoop, Transport
- `AgentCore` / `AgentCoreNative` in `agent-core` are the preferred
  application boundary for CLI, ACP, and hosted surfaces
- `SessionEngine` in `agent-runtime` is the reusable live-session substrate
- `Brain` in archived `brain-core` remains legacy engine infrastructure rather
  than the preferred product boundary
- Minimal abstractions: only add a trait when you need swappability
- `thiserror` for library errors, `anyhow` for application/example code only
- `tracing` for logging, never `println!` in library code
- Library code never reads env vars directly at runtime; file-based config
  resolution belongs in `agent-core` bootstrap / `agent-store` data layers
- Greenfield project: prefer clean design over backward-compatibility shims
- Run `just test` before landing non-trivial changes
