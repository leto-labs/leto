# Tasks: add-brain-runtime-trait

- [x] Add an OpenSpec delta for the new `BrainRuntime` trait in `brain-types`
- [x] Add `runtime.rs` to `brain-types`
- [x] Add `registry.rs` to `brain-types`
- [x] Add generic and specialized registry traits to that module
- [x] Export `BrainRuntime` from the `brain-types` crate root
- [x] Add `brain-types` tests that lock in module and root-level exports
- [x] Implement loop-aware runtime config and registry support in `brain-types`
- [x] Trim `BrainRuntime` back to runtime-owned behavior while keeping model/runtime helpers
- [x] Implement the first concrete runtime (`BrainRuntimeNative`) in `brain-core`
- [x] Migrate `brain-acp` to depend on the runtime trait where appropriate
- [x] Migrate `brain-cli` to depend on `BrainRuntime` rather than `BrainApi`
- [x] Add a live shared runtime bus to `BrainRuntime` and `BrainRuntimeNative`
- [x] Add store-emitted project/session lifecycle events and compose them through the runtime bus
- [x] Refactor `brain-types::Store` into composed project/session/message/credential accessors
- [x] Add generic keyed `CrudStore` plus specialized domain CRUD store traits in `brain-types`
- [x] Add fail-fast default batch CRUD helpers in `brain-types`
- [x] Remove `brain-server` from the active workspace while the CLI/runtime path settles
- [x] Run `cargo test --workspace`
