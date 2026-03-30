# Project: brain

## Description
Platform-agnostic AI agent engine in Rust. The current application-facing stack
uses `agent-core` over `agent-runtime`, `agent-store`, `agent-tools`,
`agent-loops`, and the standalone `provider-*` crates. Legacy `brain-*` crates
remain in the repository for compatibility and migration work.

## Architecture
- Active workspace crates:
  - `chat`
  - `chat-slack`
  - `chat-teams`
  - `chat-telegram`
  - `atif`
  - `brain-types`
  - `provider`
  - `provider-openai`
  - `provider-anthropic`
  - `provider-mistralrs`
  - `provider-llamacpp`
  - `agent-runtime`
  - `agent-loops`
  - `agent-tools`
  - `agent-store`
  - `agent-core`
  - `agent-core-remote`
  - `agent-server`
  - `agent-acp`
  - `agent-cli`
  - legacy `brain-*` compatibility crates (`brain-providers`, `brain-stores`, `brain-loops`, `brain-tools`, `brain-transports`, `brain-server`, `brain-core`, `brain-acp`, `brain-cli`, `brain-config`)
- Active workspace tool:
  - `tools/provider-preset-gen`
- Examples still present in the repo but outside the active workspace:
  - `examples/server`
- The modern stack splits responsibilities as:
  - `provider` and `provider-*` for the shared provider SDK and concrete integrations
  - `agent-runtime` for the live session engine
  - `agent-store` for durable state
  - `agent-tools` for typed tool implementations
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
- `Brain` in `brain-core` remains legacy engine infrastructure rather than the
  preferred product boundary
- Minimal abstractions: only add a trait when you need swappability
- `thiserror` for library errors, `anyhow` for application/example code only
- `tracing` for logging, never `println!` in library code
- Library code never reads env vars directly at runtime; file-based config
  resolution belongs in `agent-core` bootstrap / `agent-store` data layers
- Greenfield project: prefer clean design over backward-compatibility shims
- Run `cargo test --workspace` before landing non-trivial changes
