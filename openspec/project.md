# Project: brain

## Description
Platform-agnostic AI agent engine in Rust. A `Brain` struct orchestrates five traits (Provider, Tool, Store, AgentLoop, Transport) into a session-aware reasoning engine that emits a typed event stream. Transports abstract IO so the same engine runs behind a CLI, HTTP API, Telegram bot, or any other frontend.

## Architecture
- 8 library crates: `brain-types`, `brain-providers`, `brain-stores`, `brain-loops`, `brain-transports`, `brain-tools`, `brain-core`, `brain-server`
- Example binaries in `examples/` (`cli-echo`, `cli-local`, `oauth-login`, `server`)
- Provider backends are feature-gated and organized by runtime: `openai/` (HTTP API), `openai-oauth/`, `mistralrs/` (in-process GGUF inference), `llamacpp/` (llama.cpp inference)

## Language & Toolchain
- Rust, edition 2024, resolver 2
- Targets: native (Linux, macOS, Windows) and wasm32-unknown-unknown (future)

## Conventions
- Traits as interfaces, not class hierarchies
- Five core traits: Provider, Tool, Store, AgentLoop, Transport
- `Brain` in brain-core orchestrates all traits into a session-aware engine
- Minimal abstractions: only add a trait when you need swappability
- `thiserror` for library errors, `anyhow` for application/example code only
- `tracing` for logging, never `println!` in library code
- Library code never reads env vars — config is passed as data
