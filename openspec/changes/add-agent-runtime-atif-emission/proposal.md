## Why

The refactored `agent-*` stack can persist ATIF trajectories through
`agent-core`, but it still lacks a native ATIF emission path from
`agent-runtime`.

That leaves `agent-cli exec` rebuilding trajectories manually from terminal
output instead of consuming the same authoritative runtime records used by the
rest of the stack. It also leaves the core without a runtime-native signal for
persisting completed trajectories.

## What Changes

- add ATIF emission configuration to `agent-runtime`
- add runtime ATIF lifecycle events for trajectory start, completed steps,
  final metrics, and completed trajectories
- build ATIF trajectories natively inside `agent-runtime` from the runtime
  transcript and runtime configuration
- persist completed trajectories in `agent-core`
- update `agent-cli exec` to write `trajectory.json` from runtime ATIF events
  rather than a CLI-local manual reconstruction

## Impact

- ATIF export becomes part of the refactored runtime contract
- `agent-core` persists the same ATIF trajectories it streams to consumers
- `agent-cli exec` writes validated ATIF output from the runtime event stream
