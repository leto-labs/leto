# Proposal: add atif standalone submodule

## Why

The `atif` crate is currently embedded only inside this monorepo, which makes it
harder to evolve as a public reusable Rust package with its own repository,
README, CI, and crate metadata.

We want a standalone `atif-rust` repository that can be developed and published
independently while still being checked into this repo as a pinned Git
submodule.

## What Changes

- add `submodules/atif-rust` as an SSH Git submodule
- bootstrap the blank `atif-rust` repository as a Rust workspace
- port the existing `crates/atif` library code into the standalone workspace
- add public-package metadata, documentation, MIT licensing, and CI to the
  standalone repo
- make Harbor compatibility coverage runnable from the standalone repo via `uv`
  and a public Harbor Git dependency instead of a monorepo-local checkout

## Impact

- adds a new tracked submodule under `submodules/`
- introduces a standalone public workspace seeded from the in-tree `atif` crate
- leaves the current in-tree `crates/atif` crate untouched for now
