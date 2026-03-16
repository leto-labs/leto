## 1. brain-types: Transport trait + InputEvent
- [x] 1.1 Add `InputEvent` enum to `src/transport.rs` with `Message(String)` variant
- [x] 1.2 Add `Transport` trait to `src/transport.rs` with `name()`, `recv()`, `send()` methods
- [x] 1.3 Wire up `pub mod transport` and re-exports in `src/lib.rs`

## 2. brain-transports: New crate
- [x] 2.1 Create `crates/brain-transports/Cargo.toml` with deps on `brain-types`, `tokio` (io-util, io-std, rt, sync)
- [x] 2.2 Implement `CliTransport` in `src/cli.rs` — stdin line reader, event printer
- [x] 2.3 Wire up `src/lib.rs` with re-exports
- [x] 2.4 Add `brain-transports` to workspace `Cargo.toml` members and deps

## 3. brain-core: Brain orchestration engine
- [x] 3.1 Add `brain-transports`, `tokio`, `tokio-stream`, `futures`, `tracing`, `ulid`, `tokio-util` deps to `brain-core/Cargo.toml`
- [x] 3.2 Implement `Brain` struct in `src/brain.rs` with `new()`, `turn()`, `run()`, `create_session()`, `list_sessions()`
- [x] 3.3 Add `pub mod brain` and re-export `Brain` in `src/lib.rs`
- [x] 3.4 Add `pub use brain_transports::*` re-export in `src/lib.rs`

## 4. cli-echo: Simplify using Brain + CliTransport
- [x] 4.1 Rewrite `main.rs` to construct a `Brain` and `CliTransport`, then call `brain.run(&transport)`

## 5. Verify
- [x] 5.1 `cargo build --workspace` succeeds
- [x] 5.2 `cargo test --workspace` passes
- [x] 5.3 `cargo run -p cli-echo` works with mock provider (interactive REPL)
- [x] 5.4 `cargo run -p cli-echo` works with OpenAI provider (if OPENAI_API_KEY set)
