## Why

The refactored `provider-*` and `agent-*` stack is now the intended long-term
architecture, but this umbrella change drifted by treating `agent-server`
OpenCode compatibility work as a legacy-parity blocker.

The codebase no longer supports that framing:

- legacy `brain-server` never shipped an OpenCode compatibility layer
- legacy `brain-*` crates never implemented MCP
- the loop and terminal-session tranche tracked here is already implemented in
  the refactored stack

OpenCode compatibility remains valid current work on `agent-server`, but it is
not part of the migration baseline for deleting legacy `brain-*` crates.

## What Changes

- narrow this change to the completed advanced-loop and runtime-native
  terminal-session tranche
- record that the legacy parity baseline excludes OpenCode compatibility and
  MCP because those surfaces never existed in legacy `brain-*`
- remove the stale `agent-server` compatibility deltas so this completed change
  can be archived cleanly

## Impact

- the completed loop/runtime tranche can be archived as historical context
- the completed provider-credential and bootstrap/defaults tranches are tracked
  separately so their canonical specs can be archived independently
- OpenCode compatibility stays tracked by dedicated `agent-server` changes
  rather than the legacy-parity lane
- the remaining code-backed legacy migration follow-up is ACP-specific rather
  than `agent-server` compatibility work
