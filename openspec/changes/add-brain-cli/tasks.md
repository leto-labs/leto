# Tasks: add-brain-cli

## Implemented

- [x] Create `crates/brain-cli/` as a workspace binary crate
- [x] Remove `examples/cli-echo/` and `examples/cli-local/` from the workspace
- [x] Bootstrap an embedded `BrainRuntimeNative` for CLI use
- [x] Make the CLI depend on `Arc<dyn BrainRuntime>` rather than `BrainServer` / `BrainApi`
- [x] Resolve project roots through `runtime.resolve_or_create_project(...)`
- [x] Run interactive chat through `runtime.turn(...)`
- [x] Use `runtime.store()` for session, message, and credential CRUD
- [x] Discover providers from stored credentials, optional OAuth credentials, feature-gated local backends, and mock fallback
- [x] Use global `FileStore` rooted at `brain_home()`
- [x] Implement `brain credentials add`
- [x] Implement `brain credentials login`
- [x] Implement `brain credentials list`
- [x] Implement `brain credentials remove`
- [x] Implement `brain sessions list`
- [x] Implement `brain sessions resume <id>`
- [x] Implement `brain acp` as a stdio compatibility alias into `brain-acp`
- [x] Validate with `cargo test --workspace`

## Deferred

- [ ] Add a remote-runtime client mode once server work is redesigned around `BrainRuntime`
- [ ] Add `brain serve` on top of that remote-runtime/server path
- [ ] Add `brain attach <url>` on top of that remote-runtime/server path
- [ ] Add deeper interactive frontend work tracked outside this runtime-first CLI change
