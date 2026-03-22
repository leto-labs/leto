# Tasks: add-harbor-brain-direct-agent

- [x] Add OpenSpec deltas for the direct Harbor `brain` benchmark surface
- [x] Add OpenSpec deltas for `brain exec`
- [x] Add OpenSpec deltas for ATIF, runtime event emission, and trajectory
  persistence so the final capability layout matches the implemented
  architecture
- [x] Implement `brain exec` as a non-interactive one-shot CLI path
- [x] Add a repo-local Harbor `brain` agent and runner wiring
- [x] Default direct OpenAI-compatible `brain` runs to Responses when the
  preset supports it, with an explicit API-surface override
- [x] Add a reusable Rust `atif` crate aligned to Harbor ATIF v1.6
- [x] Emit native ATIF from Rust and persist trajectory state across turns
- [x] Keep `TurnDone` terminal while emitting ATIF completion records before it
- [x] Update Harbor and ATIF documentation for the direct `brain` surface
- [x] Run `cargo test --workspace`
- [x] Run Harbor smoke and bounded sample validation with the direct `brain`
  agent
- [x] Validate the OpenSpec change with `openspec validate add-harbor-brain-direct-agent --strict`
