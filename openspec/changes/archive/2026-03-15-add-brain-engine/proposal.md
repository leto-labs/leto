# Change: Add brain engine — core AI agent loop

## Why
We need a platform-agnostic AI reasoning engine that can later be embedded into any runtime (CLI, Tauri, server, WASM). The engine should be simple, functional, and easy to understand — not an enterprise framework.

## What Changes
- New Cargo workspace with 5 library crates + 1 orchestration engine + 1 example binary
- `brain-types`: data structs (Message, Session, Event, ToolCall, Config, errors) + 5 traits (Provider, Tool, Store, AgentLoop, Transport)
- `brain-providers`: MockProvider, OpenAiProvider (feature-gated)
- `brain-stores`: InMemoryStore, FileStore with session CRUD + message persistence
- `brain-loops`: SimpleLoop, EchoTool
- `brain-transports`: CliTransport (stdin/stdout)
- `brain-core`: orchestration engine (`Brain` struct) + re-exports all crates above
- `cli-echo` example: constructs Brain + CliTransport and runs

## Impact
- Affected specs: agent-loop, provider, tool-system, store, transport, brain-engine (all new)
- Affected code: entire workspace is new (greenfield)
- No breaking changes (nothing exists yet)
