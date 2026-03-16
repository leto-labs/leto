# Tasks: add-brain-tools

- [x] Create `crates/brain-tools/Cargo.toml` with feature-gated native deps
- [x] Add `brain-tools` to workspace `Cargo.toml` members and dependencies
- [x] Create `src/lib.rs` with module declarations and `native_tools()` preset
- [x] Move `EchoTool` from `brain-loops` to `brain-tools/src/echo.rs`
- [x] Implement `file_read/` — `FileReadDriver`, `FileReadTool<T>`, `FileReadDriverNative`
- [x] Implement `file_write/` — `FileWriteDriver`, `FileWriteTool<T>`, `FileWriteDriverNative`
- [x] Implement `file_edit/` — `FileEditDriver`, `FileEditTool<T>`, `FileEditDriverNative`
- [x] Implement `shell/` — `ShellDriver`, `ShellTool<T>`, `ShellDriverNative`
- [x] Implement `glob_search/` — `GlobDriver`, `GlobTool<T>`, `GlobDriverNative`
- [x] Implement `grep/` — `GrepDriver`, `GrepTool<T>`, `GrepDriverNative`
- [x] Update `brain-loops` to depend on `brain-tools`, re-export `EchoTool`
- [x] Update `brain-core` to depend on and re-export `brain-tools`
- [x] Update examples (`cli-echo`, `cli-local`) to use `native_tools()`
- [x] Update `AGENTS.md` crate map
- [x] Write 16 tests covering all tools
- [x] `cargo build --workspace && cargo test --workspace` — 28 tests pass
