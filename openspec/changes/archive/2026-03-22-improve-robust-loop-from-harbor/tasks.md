# Tasks: improve-robust-loop-from-harbor

- [x] Expand `docs/benchmarks/LoopDesign.md` with the Harbor-driven loop analysis
- [x] Update the ACP backend so `MaxIterations` and `Cancelled` stop surfacing
  as ACP internal failures
- [x] Add OpenSpec deltas for ACP surfacing of non-internal loop termination
- [x] Validate the change with `openspec validate improve-robust-loop-from-harbor --strict`
