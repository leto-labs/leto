- [x] Add OpenSpec capability deltas for `agent-runtime` and `agent-loops`
- [x] Add `agent-runtime` crate and workspace wiring
- [x] Define runtime config, errors, session state, runtime events, tool executor,
  loop strategy, and session engine APIs
- [x] Implement provider-turn orchestration and tool execution helpers in
  `agent-runtime`
- [x] Add `agent-loops` crate and implement `SimpleLoop`
- [x] Refactor `agent-runtime` into a bidirectional session engine with session
  commands, control events, boundaries, and snapshots
- [x] Add a flat internal runtime registry for live runtime lookup, parent-child
  lineage, and parent-child message routing
- [x] Replace the per-turn loop trait with a decision-oriented strategy surface
- [x] Port `SimpleLoop` to the new runtime and keep it within reusable runtime
  boundaries
- [x] Tighten the runtime API around typed agent actions and non-blocking
  defaults
- [x] Add provider-visible native agent tools for spawn, message, read mail,
  interrupt, list, and wait
- [x] Add registry-routed direct vs mailbox cross-agent messaging and
  provider-visible child report injection
- [x] Add unit tests covering steering, interruption, approval pause/resume,
  child spawn/listing, registry-routed agent messaging, direct-message
  injection, mailbox reads, snapshots, and simple tool-loop re-entry on the new
  runtime
- [x] Validate the change with `openspec validate add-agent-runtime-and-loops --strict`
- [x] Run targeted cargo tests for the new crates
