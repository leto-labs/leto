# Tasks: add-session-inference-overrides

- [x] Add an OpenSpec change for session-scoped inference overrides and provider-aware routing
- [x] Extend `Session` and `SessionUpdate` to support persisted inference overrides
- [x] Update store implementations to persist session inference changes
- [x] Move effective session inference resolution into `brain-core::Brain`
- [x] Change `Brain::turn()` to load session/project config internally
- [x] Redesign `ProviderRouter` to route by provider first and by model only when unambiguous
- [x] Update provider builders to register named providers instead of model-only routes
- [x] Implement real ACP session model state and `session/set_model`
- [x] Add tests for session inference persistence, effective merge behavior, router resolution, and ACP model switching
- [x] Run `cargo test --workspace`
