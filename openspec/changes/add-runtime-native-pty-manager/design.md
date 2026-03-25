# Design: Runtime-Native PTY Manager

## Core Rule

PTI/terminal sessions should be modeled as runtime-managed resources:

- stable `PtyId`
- explicit lifecycle
- structured snapshots and execution results
- explicit event subscriptions
- optional safe-boundary transcript promotion

This keeps PTY aligned with the newer runtime-native direction for child
runtimes and messaging.

## Runtime Shape

The runtime owns:

- PTY registry/storage
- PTY lifecycle and status
- buffered output and snapshot state
- execution results
- event emission
- subscription delivery
- safe-boundary developer-message promotion

Loops own:

- when to open or reuse a PTY
- when to execute or capture
- when to subscribe and what to promote
- how to combine PTY observations with subcalls, rewrites, and other runtime
  effects

## Delivered Surface

This change adds:

- typed PTY runtime actions
- PTY loop effects
- PTY session state in `SessionState`
- PTY runtime events
- provider-visible native PTY tools

The implementation intentionally stays bounded:

- no loop DSL
- no plan-level branching
- PTY events are runtime facts first and transcript content only through
  explicit subscription with promotion

## Event Model

PTYs emit runtime events such as:

- output
- execution started/completed
- status changed
- resized
- interrupted
- closed
- failed

Subscriptions filter by PTY id and event kinds. Delivery may remain
runtime-visible only or promote matching events into `developer` messages at
safe boundaries.
