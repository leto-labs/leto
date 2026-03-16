## 1. Dependencies
- [x] 1.1 Add `mistralrs` to workspace `Cargo.toml` deps
- [x] 1.2 Add `local` feature to `brain-providers/Cargo.toml` with `dep:mistralrs`

## 2. Local provider implementation
- [x] 2.1 Create `crates/brain-providers/src/local/config.rs` with `LocalConfig` and `DevicePreference`
- [x] 2.2 Create `crates/brain-providers/src/local/presets.rs` with `LocalModelPreset` and Qwen 3 presets
- [x] 2.3 Create `crates/brain-providers/src/local/provider.rs` with `LocalProvider` implementing `Provider` trait
- [x] 2.4 Create `crates/brain-providers/src/local/mod.rs` wiring up the module
- [x] 2.5 Wire up feature-gated `local` module in `brain-providers/src/lib.rs`

## 3. Feature propagation
- [x] 3.1 Add `local` feature to `brain-core/Cargo.toml` that forwards to `brain-providers/local`
- [x] 3.2 Update `brain-core/src/lib.rs` to conditionally re-export local types

## 4. Example binary
- [x] 4.1 Create `examples/cli-local/Cargo.toml` depending on brain-core with `local` feature
- [x] 4.2 Create `examples/cli-local/src/main.rs` with LocalProvider REPL
- [x] 4.3 Add `cli-local` to workspace members

## 5. Smoke test
- [x] 5.1 Create `crates/brain-providers/tests/local_smoke.rs` with Qwen 0.6B test

## 6. Verify
- [x] 6.1 `cargo build --workspace --exclude cli-local` succeeds (without local feature)
- [x] 6.2 `cargo check -p brain-providers --features local` succeeds
- [x] 6.3 Test compilation and unit tests pass
