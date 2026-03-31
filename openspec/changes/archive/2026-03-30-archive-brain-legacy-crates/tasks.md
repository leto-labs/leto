# Tasks: archive-brain-legacy-crates

## Implementation

- [x] Add a gitignored local archive path for legacy `brain-*` source
- [x] Move committed `crates/brain-*` source into the local archive tree
- [x] Remove committed workspace membership and build/tooling references to the
      archived crates
- [x] Archive legacy-only examples, scripts, and Harbor helpers that still
      require `brain-*`
- [x] Retire canonical `brain-*` specs from `openspec/specs/` into a
      historical archive tree
- [x] Archive or retire active OpenSpec changes that still target removed
      legacy capabilities
- [x] Validate the change with `openspec validate
      archive-brain-legacy-crates --strict`
- [x] Run `cargo build --workspace` and `cargo test --workspace`
- [x] Run `openspec validate --specs`
