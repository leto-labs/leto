# Project: brain

## Description
Platform-agnostic AI agent engine in Rust. `Brain` remains the reusable engine
API, while `BrainRuntime` is the preferred application boundary for local and
protocol-facing surfaces such as the CLI and ACP. The system is built around
five core abstractions: Provider, Tool, Store, AgentLoop, and Transport.

## Architecture
- Active workspace crates:
  - `brain-types`
  - `provider`
  - `provider-openai`
  - `provider-anthropic`
  - `provider-mistralrs`
  - `provider-llamacpp`
  - `brain-providers`
  - `brain-stores`
  - `brain-loops`
  - `brain-tools`
  - `brain-transports`
  - `brain-core`
  - `brain-acp`
  - `brain-cli`
  - `brain-config`
- Active workspace tool:
  - `tools/provider-preset-gen`
- Additional repo crates currently kept out of the active workspace:
  - `brain-server`
- Examples still present in the repo but outside the active workspace:
  - `examples/server`
- Provider backends are feature-gated and organized by runtime:
  - `openai/` for OpenAI-compatible HTTP APIs
  - `openai_oauth/` for subscription-backed OAuth access
  - `mistralrs/` for in-process GGUF inference
  - `llamacpp/` for llama.cpp inference
  - standalone `provider-*` crates for shared v2 provider integrations

## Language & Toolchain
- Rust, edition 2024, resolver 2
- Targets: native (Linux, macOS, Windows) and wasm32-unknown-unknown (future)

## Conventions
- Traits as interfaces, not class hierarchies
- Five core traits: Provider, Tool, Store, AgentLoop, Transport
- `Brain` in `brain-core` is the reusable engine API
- `BrainRuntime` / `BrainRuntimeNative` are the preferred runtime boundary for
  CLI and ACP surfaces
- Minimal abstractions: only add a trait when you need swappability
- `thiserror` for library errors, `anyhow` for application/example code only
- `tracing` for logging, never `println!` in library code
- Library code never reads env vars directly at runtime; file-based config
  resolution belongs in `brain-config` or application bootstrap layers
- Greenfield project: prefer clean design over backward-compatibility shims
- Run `cargo test --workspace` before landing non-trivial changes
