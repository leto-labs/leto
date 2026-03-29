- [x] Add ATIF emission configuration and event variants to `agent-runtime`.
- [x] Port ATIF trajectory construction into `agent-runtime` and emit ATIF
      lifecycle events during turns.
- [x] Persist completed trajectories from `agent-core` using the emitted runtime
      events.
- [x] Update `agent-cli exec` to consume runtime ATIF events and write validated
      `trajectory.json`.
- [x] Validate the OpenSpec change with `openspec validate
      add-agent-runtime-atif-emission --strict`.
- [x] Run `cargo check -p agent-runtime`.
- [x] Run `cargo check -p agent-cli`.
