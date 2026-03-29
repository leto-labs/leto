# Add agent-cli

## Why

The repository is migrating user-facing surfaces from legacy `brain-*` crates
to the refactored `agent-*` stack.

`brain-cli` still owns the local interactive/admin CLI, but it embeds the
legacy `BrainRuntime` path and the legacy config/bootstrap composition. We want
the local CLI to move onto `AgentCore` so credentials, project bootstrap, and
session execution all use the refactored stack directly.

## What Changes

- add a new `agent-cli` workspace binary crate
- define `agent-cli` as the canonical local CLI on the refactored stack
- move `.agents/config.toml` and `.agents/AGENTS.md` bootstrap resolution into
  `agent-store`
- have `agent-core` consume the store-owned bootstrap loader when creating new
  projects
- implement interactive chat, session management, and credential management on
  top of `AgentCore`

## Impact

- adds a new user-facing CLI crate in the `agent-*` stack
- reduces local CLI dependence on legacy `brain-*` runtime embedding
- makes project bootstrap behavior reusable between `agent-core` and future
  `agent-*` surfaces
