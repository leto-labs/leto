## 1. Implementation
- [x] 1.1 Create `crates/brain-providers/src/presets.rs` with `OpenAiConfigPreset` struct, 9 built-in presets, `ALL` slice, `by_name()` lookup
- [x] 1.2 Wire up `pub mod presets` and re-export `OpenAiConfigPreset` in `brain-providers/src/lib.rs`
- [x] 1.3 Update `OpenAiConfig::new()` to source defaults from `OpenAiConfigPreset::OPENAI`
- [x] 1.4 Update `cli-echo/src/main.rs` to use `OpenAiConfigPreset::by_name()` with `PROVIDER` env var

## 2. Verify
- [x] 2.1 `cargo build --workspace` succeeds
- [x] 2.2 `cargo test --workspace` passes
- [x] 2.3 `cargo run -p cli-echo` works with default (openai) preset
- [x] 2.4 Verify `PROVIDER` env var selects the correct preset
