# Proposal: Add Runtime-Native PTY Manager

## Why

`agent-runtime` now has strong native support for child runtimes, waiting, and
cross-agent messaging, but persistent terminal execution still lives outside
that model.

Advanced loops such as Terminus2 and Terminus Kira need more than one-shot
shell commands:

- stable PTY/session identity
- persistent terminal state across many turns
- explicit interrupt/background/close behavior
- incremental output capture
- event subscriptions
- optional promotion of selected PTY events into model-visible `developer`
  messages

Treating PTY as a generic tool hides this state inside a tool backend and makes
it harder for loops and the runtime to reason about terminal execution as a
first-class capability.

## What Changes

- add a runtime-managed PTY subsystem to `agent-runtime`
- expose typed PTY commands for open/list/get/write/execute/capture/resize/
  interrupt/background/close
- expose PTY subscriptions and runtime-visible PTY event delivery
- allow explicitly subscribed PTY events to be promoted into transcript
  `developer` messages at safe boundaries
- expose provider-visible native PTY tools mirroring the typed runtime actions

## Impact

- PTY becomes a runtime capability parallel to subagent control rather than a
  legacy tool concern
- advanced loops can reason over structured PTY state and event streams
- the runtime gets a cleaner path toward Terminus-style and Kira-style terminal
  behavior without adding a loop DSL
