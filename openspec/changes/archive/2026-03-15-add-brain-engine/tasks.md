## 1. Workspace Scaffold
- [x] 1.1 Create workspace root `Cargo.toml` (edition 2024, resolver 2, workspace deps)
- [x] 1.2 Create `rust-toolchain.toml` pinning stable channel
- [x] 1.3 Create crate directories: brain-types, brain-providers, brain-stores, brain-loops, brain-core, cli-echo
- [x] 1.4 Create `Cargo.toml` for each crate with workspace dependency inheritance

## 2. brain-types (data + traits)
- [x] 2.1 Implement `Message`, `Role` in `src/message.rs`
- [x] 2.2 Implement `Event` enum in `src/event.rs`
- [x] 2.3 Implement `ToolCall`, `ToolDef` in `src/tool.rs`
- [x] 2.4 Implement `AgentConfig`, `InferenceConfig`, `TokenUsage` in `src/config.rs`
- [x] 2.5 Implement `Session`, `SessionUpdate` in `src/session.rs`
- [x] 2.6 Implement `BrainError`, `BrainErrorCode` in `src/error.rs`
- [x] 2.7 Implement `ChatChunk` in `src/stream.rs`
- [x] 2.8 Define `Provider` trait + `ChatStream` type alias in `src/provider.rs`
- [x] 2.9 Define `Tool`, `Store`, `AgentLoop` traits + `EventStream` type alias in `src/traits.rs`
- [x] 2.10 Wire up `src/lib.rs` with re-exports

## 3. brain-providers
- [x] 3.1 Implement `MockProvider` in `src/mock.rs`
- [x] 3.2 Implement `OpenAiProvider` + `OpenAiConfig` in `src/openai.rs` (feature-gated)
- [x] 3.3 Create `openai_types.rs` for request/response serde structs
- [x] 3.4 Wire up `src/lib.rs` with feature-gated re-exports

## 4. brain-stores
- [x] 4.1 Implement `InMemoryStore` in `src/memory.rs` (session CRUD + message ops)
- [x] 4.2 Implement `FileStore` in `src/file.rs` (JSON session + JSONL messages, directory-per-session)
- [x] 4.3 Wire up `src/lib.rs`

## 5. brain-loops
- [x] 5.1 Implement `SimpleLoop` in `src/simple.rs`
- [x] 5.2 Implement `EchoTool` in `src/echo_tool.rs`
- [x] 5.3 Wire up `src/lib.rs`

## 6. brain-core (facade)
- [x] 6.1 Re-export brain-types, brain-providers, brain-stores, brain-loops

## 7. cli-echo Example
- [x] 7.1 Implement interactive REPL with MockProvider/OpenAiProvider selection
- [x] 7.2 Load `.env` via dotenvy
- [x] 7.3 Verify `cargo build --workspace` succeeds
- [x] 7.4 Verify `cargo run -p cli-echo` works with mock and real OpenAI
- [x] 7.5 Verify `cargo test --workspace` passes
