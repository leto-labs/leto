## 1. Type Changes
- [x] 1.1 Change `InferenceConfig.model_id: String` to `model: Option<String>`
- [x] 1.2 Add `inference: InferenceConfig` to `AgentConfig`
- [x] 1.3 Update `SimpleLoop` to use `config.inference` instead of `InferenceConfig::default()`

## 2. Dependencies
- [x] 2.1 Add `reqwest` (rustls-tls, json, stream) and `eventsource-stream` to workspace deps
- [x] 2.2 Add feature-gated optional deps in brain-providers Cargo.toml

## 3. OpenAI Provider (in brain-providers)
- [x] 3.1 Create `openai_types.rs` with request/response serde structs
- [x] 3.2 Create `openai.rs` with `OpenAiConfig` and `OpenAiProvider`
- [x] 3.3 Implement message/tool mapping (our types to OpenAI API format)
- [x] 3.4 Implement SSE stream parsing with tool call accumulation
- [x] 3.5 Wire up in `lib.rs` behind `#[cfg(feature = "openai")]`

## 4. CLI Update
- [x] 4.1 Update cli-echo to detect `OPENAI_API_KEY` and create real provider
- [x] 4.2 Add dotenvy for `.env` loading
- [x] 4.3 Add interactive REPL loop with stdin/stdout
- [x] 4.4 Verify mock mode still works: `echo "hello" | cargo run -p cli-echo`

## 5. Validation
- [x] 5.1 `cargo build --workspace` succeeds
- [x] 5.2 `cargo test --workspace` passes
- [x] 5.3 Manual test with real API key via `.env` confirmed working
