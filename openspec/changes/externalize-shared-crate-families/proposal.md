# Change: Externalize shared crate families

## Why
- `atif`, `provider-*`, and `chat-*` now have standalone upstream repositories
  and should be developed in isolation from the `leto` application workspace.
- Keeping duplicate in-tree copies inside `leto` creates drift and makes it
  unclear which repository is the source of truth.
- The repo already vendors standalone work through pinned submodules, so the
  same model fits these reusable crate families.

## What
- Vendor `ai-provider-rs` and `chat-rs` as pinned SSH submodules alongside the
  existing `atif-rust` submodule.
- Switch the `leto` root workspace to consume `atif`, `provider-*`, and
  `chat-*` through local `path` dependencies into those submodules.
- Remove the in-tree copies of `crates/atif`, `crates/chat*`,
  `crates/agent-provider/provider*`, and `tools/provider-preset-gen`.
- Update repo tooling and contributor docs to use explicit multi-workspace
  commands now that those crates are no longer root workspace members.

## Impact
- Root `cargo test --workspace` only covers the `leto` workspace crates.
- Vendored submodules become the only local source of truth for `atif`,
  `provider-*`, `chat-*`, and provider preset generation.
- Repo tooling and specs need to reflect the new submodule-based topology.
