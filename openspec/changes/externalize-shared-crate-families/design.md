# Design: Externalize shared crate families

## Summary
`leto` keeps the application-facing runtime crates in the root workspace while
vendoring reusable crate families through pinned Git submodules:

- `submodules/atif-rust`
- `submodules/ai-provider-rs`
- `submodules/chat-rs`

The root workspace depends on those crates through local `path` dependencies
and explicitly excludes the submodule roots so Cargo resolves each vendored
workspace against its own nested `[workspace]`.

## Key decisions

### Exclude vendored submodules from the root workspace
Cargo will otherwise try to associate nested manifests with the outer workspace,
which breaks workspace inheritance in the vendored crates. The root workspace
therefore uses explicit `exclude` entries for the vendored submodule roots.

### Preserve package names and Rust import surfaces
The migration changes only repository topology and local dependency paths.
Existing package names remain:

- `atif`
- `chat`, `chat-slack`, `chat-teams`, `chat-telegram`
- `provider`, `provider-openai`, `provider-anthropic`,
  `provider-mistralrs`, `provider-llamacpp`

That keeps downstream `workspace = true` consumers in `leto` unchanged.

### Treat the vendored repos as the only source of truth
The in-tree copies are removed after the root workspace is rewired to the
submodules. `tools/provider-preset-gen` is removed from `leto` because the
canonical generator now lives in `submodules/ai-provider-rs/tools/`.

### Make repo commands explicitly multi-workspace
The root `just` recipes run the root workspace first and then each vendored
workspace through `--manifest-path`. Coverage remains root-workspace only for
now; the repo docs call that out explicitly.
