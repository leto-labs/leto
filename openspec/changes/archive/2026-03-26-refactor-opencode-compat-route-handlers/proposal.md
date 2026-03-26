# Proposal: refactor-opencode-compat-route-handlers

## Why

The OpenCode compat subtree has already been split into route-family files, but
the actual HTTP-facing handler bodies still lived centrally in
`compat/opencode/mod.rs`. That made the module layout misleading and forced
changes to jump between route registration files and a monolithic handler file.

Now that the compatibility contract and exact `/doc` parity are stable, the
compat implementation should match the route-family structure directly.

## What

This change moves every HTTP-facing compat handler into its corresponding
`routes/*.rs` file and reduces `compat/opencode/mod.rs` to shared
infrastructure only.

The refactor preserves:

- exact prefix-aware `/v1/compat/opencode/doc` parity
- current compat behavior
- current canonical `/v1` behavior

## Impact

- Modified capability: `agent-server`
- Internal compat module cleanup only; no intended wire-contract changes
- Stronger locality between routes, handlers, and route-family docs
