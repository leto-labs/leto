## Context

`brain-core` is a pure re-export facade. The orchestration that ties Provider + Store + AgentLoop + Tools together lives entirely in `cli-echo/src/main.rs`. Every new frontend would duplicate this wiring. Additionally, IO is hardcoded to stdin/stdout with no abstraction for alternative transports (Telegram, HTTP API, etc.).

Researched ZeroClaw (Rust, trait-driven channels), OpenClaw (TypeScript, plugin-based channels), and OpenCode (Go/TS, server/client model) for transport abstraction patterns.

## Goals
- `brain-core` becomes the orchestration engine with a `Brain` struct
- New `Transport` trait abstracts IO (input events in, output events out)
- `CliTransport` as the first implementation
- `cli-echo` reduced to minimal wiring (~15 lines)

## Non-Goals
- No multi-channel multiplexing (single transport per Brain.run())
- No Telegram/Discord/API transports yet (just the trait + CLI)
- No authentication or permission system for transports
- No breaking changes to existing AgentLoop, Provider, Store, or Tool traits

## Decisions

### 1. Transport trait modeled after ZeroClaw's Channel

**ZeroClaw** defines a `Channel` trait with `send(SendMessage)` and `listen(tx: mpsc::Sender<ChannelMessage>)`. Each platform (Telegram, Discord, CLI) implements `Channel`. A central MPSC bus collects all inbound messages, and a dispatch loop spawns workers per message.

**Our adaptation**: ZeroClaw targets multi-channel bots with a global message bus. We target session-based agents where one transport drives one Brain at a time. We simplify to a `recv/send` pair without the MPSC bus:

```rust
pub enum InputEvent {
    Message(String),
}

pub trait Transport: Send + Sync {
    fn name(&self) -> &str;
    fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>>;
    fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>>;
}
```

`recv()` returns `None` on EOF/shutdown for clean exit. `send()` dispatches each `Event` to the transport for rendering. The transport owns its own IO resources (file handles, HTTP connections, etc.).

**Why not ZeroClaw's listen(tx) pattern?** Their pattern is designed for long-running listeners that push into a shared bus. Ours is pull-based: the Brain calls `recv()` when it's ready for the next input. This avoids the complexity of a dispatch loop and backpressure management while remaining easy to extend later.

### 2. Brain struct as the orchestration engine

`brain-core` stops being a pure facade and gains a `Brain` struct:

```rust
pub struct Brain {
    pub provider: Arc<dyn Provider>,
    pub store: Arc<dyn Store>,
    pub agent_loop: Arc<dyn AgentLoop>,
    pub tools: Vec<Arc<dyn Tool>>,
    pub config: AgentConfig,
}
```

Two main methods:
- `turn(session_id, input, cancel) -> EventStream` — single-turn: loads history from store, appends user message, runs agent loop, persists new messages, returns event stream
- `run(transport) -> Result<()>` — main loop: creates a session, repeatedly calls `recv()` on the transport, dispatches turns, forwards events via `send()`

`brain-core` continues to re-export all sub-crates so downstream consumers can still `use brain_core::*`.

### 3. New crate: brain-transports

Follows the existing pattern (brain-providers, brain-stores, brain-loops). Contains Transport implementations:

- `CliTransport`: reads lines from stdin via `tokio::io::BufReader<Stdin>`, writes events to stdout/stderr. Prints prompt on stderr, token deltas to stdout, tool call info to stdout.

### 4. Crate dependency graph

```
brain-types          (traits + data, no deps on other brain crates)
brain-providers      (depends on brain-types)
brain-stores         (depends on brain-types)
brain-loops          (depends on brain-types)
brain-transports     (depends on brain-types)
brain-core           (depends on all above, owns Brain struct)
cli-echo             (depends on brain-core)
```

### 5. Workspace layout after change

```
crates/
  brain-types/        -- data structs + 5 traits (Provider, Tool, Store, AgentLoop, Transport)
  brain-providers/    -- MockProvider, OpenAiProvider
  brain-stores/       -- InMemoryStore, FileStore
  brain-loops/        -- SimpleLoop, EchoTool
  brain-transports/   -- CliTransport
  brain-core/         -- Brain struct + re-exports
examples/
  cli-echo/           -- minimal wiring using Brain + CliTransport
```

## Tech stack additions

| Crate | Purpose |
|---|---|
| tokio (io-util feature) | Async stdin reading in CliTransport |

## Risks / Trade-offs
- Pull-based `recv()` means the Brain blocks waiting for input between turns — acceptable for CLI, fine for request-response transports (HTTP), but push-based transports (Telegram webhooks) will need to buffer internally and return from `recv()` when a message arrives
- Single transport per `Brain.run()` — no fan-out to multiple channels. Multi-channel support can be added later with a `MultiplexTransport` that wraps multiple transports behind a single recv/send interface
