# brain

Platform-agnostic AI agent engine in Rust. The autonomous reasoning core that runs agent loops, dispatches tools, manages context, and emits a typed event stream.

Zero knowledge of any UI, platform API, or runtime environment. All platform capabilities are injected as trait implementations.

## Quick Start

```bash
cargo run -p cli-echo
```

Output:
```
hello world
[done in 1 iteration(s)]
```

## Architecture

Four traits. Four implementations. One agent loop.

```
Provider  ──→  AgentLoop  ──→  Stream<Event>
Tool      ──↗
```

| Trait | What | Default Implementation |
|---|---|---|
| `Provider` | Talk to an LLM, get a stream of chunks | `MockProvider` (echo) |
| `Tool` | Define schema + execute | `EchoTool` (log and return) |
| `Store` | Session CRUD + message persistence | `InMemoryStore` (HashMap) |
| `AgentLoop` | Orchestration strategy | `SimpleLoop` (tool loop) |

The agent loop is a **trait**, not a hardcoded function. Different strategies are different implementations:
- `SimpleLoop` — call provider, execute tool calls, loop until done
- `PlanLoop` — think first with no tools, then execute *(future)*
- `ExploreLoop` — read-only search and analysis *(future)*

## Workspace

```
crates/
  provider/           Shared v2 provider SDK
  provider-openai/    Standalone OpenAI-compatible v2 provider crate
  provider-anthropic/ Standalone Anthropic v2 provider crate
  provider-mistralrs/ Standalone mistral.rs local v2 provider crate
  provider-llamacpp/  Standalone llama.cpp local v2 provider crate
  agent-runtime/      Live per-session execution engine
  agent-loops/        Current loop strategies
  agent-tools/        Typed tool implementations and native tool registry
  agent-store/        Durable project/session/message/credential storage
  agent-core/         Shared product-facing orchestration boundary
  agent-core-remote/  Remote AgentCore client
  agent-server/       Hosted server surface
  agent-acp/          ACP adapter surface
  agent-cli/          Local CLI surface
```

## Tech Stack

| Concern | Choice |
|---|---|
| Edition | 2024 (Rust 1.85+, native async fn in trait) |
| Async | tokio (selective features) |
| Serialization | serde + serde_json |
| Errors | thiserror (library), anyhow (apps only) |
| Logging | tracing |
| IDs | ulid (time-sortable) |
| Streaming | futures + async-stream + tokio channels |

## Provider Roadmap

The current provider path is centered on `provider` plus standalone
`provider-*` crates:

1. **MockProvider** *(done)* — echo, no network, no API keys
2. **OpenAiProvider** *(done)* — reqwest + SSE streaming, feature-gated behind `openai`
3. **Ollama** — use OpenAiProvider with `with_base_url("http://localhost:11434/v1")`
4. **Local** — Candle / llama.cpp bindings, fully offline
5. **WASM** — a future browser-facing adapter over the modern stack

## Build Commands

```bash
cargo run -p cli-echo                              # mock provider (no API key needed)
OPENAI_API_KEY=sk-... cargo run -p cli-echo        # real OpenAI inference
OPENAI_MODEL=gpt-4o cargo run -p cli-echo          # custom model
cargo test --workspace                              # run all tests
cargo build --release                               # release build
cargo check -p provider-mistralrs --features mistralrs
cargo check -p provider-llamacpp --features llamacpp
```

## Spec-Driven Development

This project uses [OpenSpec](openspec/) for managing specifications and change proposals.

```bash
ls openspec/changes/             # active proposals
ls openspec/specs/               # current truth (built capabilities)
```

For integration and client-validation work that depends on checked-out forks,
initialize the repository submodules locally:

```bash
git submodule update --init --recursive
```

## License

TBD
## Development Hooks

This repository uses committed [`lefthook`](https://lefthook.dev/) hook
configuration for local verification.

After cloning, install the hooks locally:

```bash
lefthook install
```

Configured hooks:

- `pre-commit`
  - format staged Rust files with `rustfmt`
  - restage any formatting changes automatically

The hook intentionally does not run `cargo fmt --all`. It only formats the Rust
files currently staged for commit.
