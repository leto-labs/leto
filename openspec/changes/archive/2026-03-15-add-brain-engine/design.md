## Context
Building the foundational AI agent engine from scratch. Informed by analysis of zeroclaw (Rust, trait-driven), ironclaw (Rust, security-focused), rig (Rust, WASM-compatible), async-openai (Rust, SSE streaming), opencode (Go, server/client), and oh-my-pi (TypeScript, layered agent architecture).

## Goals
- Functional, easy-to-understand architecture
- The agent loop is a trait, different strategies are different implementations
- Five traits: Provider, Tool, Store, AgentLoop, Transport
- Compiles and runs with `cargo run -p cli-echo` on first build
- Everything testable without API keys or network

## Non-Goals
- No HTTP server (transport is a future concern)
- No WASM bindings yet (just ensure core compiles to wasm32)
- No permissions/approval flow (add later when needed)
- No context compaction (add later)
- No SQLite store yet (InMemoryStore and FileStore only for now)

## Decisions

### 1. Edition 2024 with native async fn in trait
- No `async-trait` crate needed
- Requires Rust 1.85+
- Cleaner, zero-cost

### 2. AgentLoop as a trait, not a hardcoded function
- Different agent strategies are different implementations: SimpleLoop, PlanLoop, ExploreLoop, etc.
- `SimpleLoop` is the default: call provider, execute tools, loop
- Caller owns the messages and persistence
- No BrainBuilder with 7 required fields

### 3. Five traits
- `Provider`: send messages + tool schemas, get back a stream of chunks
- `Tool`: definition + execute
- `Store`: session CRUD + message persistence
- `AgentLoop`: the orchestration strategy (SimpleLoop, PlanLoop, etc.)
- `Transport`: IO abstraction (recv user input, send engine events)
- `Brain` in brain-core orchestrates them all

### 4. Tech stack
| Crate | Version | Purpose |
|---|---|---|
| tokio | 1 | Async runtime (selective features) |
| serde + serde_json | 1 | Serialization |
| thiserror | 2 | Library errors |
| tracing | 0.1 | Logging |
| ulid | 1 | Time-sortable IDs |
| chrono | 0.4 | Timestamps |
| futures | 0.3 | Stream combinators |
| async-stream | 0.3 | Stream construction in agent loop |
| tokio-util | 0.7 | CancellationToken |

### 5. Workspace layout
```
Cargo.toml
crates/
  brain-types/        -- data structs + 5 traits (Provider, Tool, Store, AgentLoop, Transport)
  brain-providers/    -- MockProvider, OpenAiProvider (feature-gated)
  brain-stores/       -- InMemoryStore, FileStore
  brain-loops/        -- SimpleLoop, EchoTool
  brain-transports/   -- CliTransport
  brain-core/         -- Brain orchestration engine + re-exports all crates above
examples/
  cli-echo/           -- constructs Brain + CliTransport, calls brain.run()
```

## Risks / Trade-offs
- Edition 2024 requires Rust 1.85+ which is recent — mitigated by rust-toolchain.toml
- No permissions/approval means tools always auto-execute — acceptable for mock/dev phase
- InMemoryStore loses data on exit — FileStore added for persistence

## Open Questions
- None blocking initial implementation
