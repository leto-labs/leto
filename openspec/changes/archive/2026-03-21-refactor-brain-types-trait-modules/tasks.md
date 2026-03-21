# Tasks: refactor-brain-types-trait-modules

- [x] Add an OpenSpec delta describing the dedicated trait modules in
      `brain-types`
- [x] Move the `Tool` trait into `tool.rs`
- [x] Add `store.rs` and move all store-related traits into it
- [x] Add `agent_loop.rs` and move `EventStream` and `AgentLoop` into it
- [x] Update `brain-types` module declarations and root re-exports
- [x] Add a `brain-types` public API test covering the new module paths
- [x] Format the touched Rust files
- [x] Run `cargo test --workspace`
