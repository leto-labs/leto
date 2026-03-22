# Update Harbor Brain ACP To Use Portable Linux Artifacts

## Why

The current Harbor `brain-acp` path stages a locally built host binary into
Terminal Bench task images. On this machine that default artifact is
`target/debug/brain-acp`, which is dynamically linked against host glibc 2.39.
Full `terminal-bench@2.0` includes older Debian/bookworm-based images, so the
backend fails before ACP initialization.

The Harbor runner also currently defaults `HARBOR_N_TASKS=1`, which means a
dataset-only invocation does not actually run the full dataset unless the user
knows to override it.

## What Changes

- add a repo-supported musl-targeted Harbor build path for `brain-acp`
- make `AcpBrainAgent` prefer the portable Harbor artifact before host-glibc
  artifacts
- update `just build` and `just build-release` to target musl explicitly
- make `scripts/harbor-run.sh` run the full dataset by default when no
  `task-name` is provided
- document the musl toolchain requirement and updated Harbor runner semantics

## Impact

- contributors get an explicit portable Linux artifact path for Harbor
- Harbor `brain-acp` runs become compatible with older task images
- dataset-wide benchmark launches behave the way users expect
