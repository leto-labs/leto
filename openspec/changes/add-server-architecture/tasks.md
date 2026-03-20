# Tasks: add-server-architecture

## Deferred Remote-Runtime Work

- [ ] Define the remote runtime contract around `BrainRuntime` parity rather than `BrainApi`
- [ ] Define proxy `Store` and registry implementations for a future `BrainRuntimeApi`
- [ ] Rework `brain-server` so it hosts `BrainRuntimeNative` rather than acting as a separate primary client boundary
- [ ] Define the transport for remote turn streams and live runtime-bus subscription
- [ ] Define remote handling for store-backed project, session, message, and credential operations
- [ ] Restore `brain-server` and `examples/server` to the active workspace once that design is ready
- [ ] Update `brain-cli` remote modes (`serve`, `attach`) on top of the remote-runtime path
