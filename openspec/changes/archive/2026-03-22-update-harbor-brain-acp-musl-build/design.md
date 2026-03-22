# Design

## Portable Artifact Strategy

We will follow the same high-level packaging idea used by Codex:

- normal local dev commands remain ordinary Rust builds
- the Harbor artifact path uses an explicit Linux target intended for
  portability

For this repo the first portable target is:

- `x86_64-unknown-linux-musl`

We are not changing the entire workspace architecture around release packaging.
We are only making the existing repo build surface and Harbor ACP artifact
lookup prefer a portable Linux binary.

## Build Surface

The user explicitly wants `build` and `build-release` to target musl. That will
be implemented in `justfile`.

Because the current host does not have a musl C toolchain installed, the build
commands must also check for `musl-gcc` and fail with a clear instruction when
it is missing.

We will also add a target-specific Cargo config for:

- `x86_64-unknown-linux-musl`

This config should remain narrow and only declare the target linker.

## Harbor Artifact Resolution

`AcpBrainAgent` currently defaults to:

1. `target/debug/brain-acp`
2. `target/release/brain-acp`

That order is wrong for Harbor because it prefers the least portable binary.
The new order will be:

1. `target/x86_64-unknown-linux-musl/release/brain-acp`
2. `target/release/brain-acp`
3. `target/debug/brain-acp`

This keeps override support intact while making the safe artifact the default.

## Harbor Runner Semantics

The current runner always defaults `HARBOR_N_TASKS=1`, which is correct for
single-task ladder checkpoints but wrong for dataset-only invocations.

The new rule is:

- if `task-name` is present and `HARBOR_N_TASKS` is unset, default to `1`
- if `task-name` is absent and `HARBOR_N_TASKS` is unset, do not pass
  `--n-tasks`

This preserves bounded runs while making full-suite runs natural.
