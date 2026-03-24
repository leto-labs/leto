# Change: Add standalone local provider crates

## Why
- `brain-providers` still owns the local `llamacpp` and `mistralrs` implementations.
- The v2 shared `provider` SDK now has standalone wire/provider crates for remote backends, but local backends still depend on `brain-types`.
- We need standalone local provider crates that can be reused without pulling in any `brain-*` crate.

## What
- Add `provider-llamacpp` as a standalone crate for llama.cpp-backed local inference.
- Add `provider-mistralrs` as a standalone crate for mistral.rs-backed local inference.
- Port the existing config/preset/lazy-load/preload/local-inference behavior out of `brain-providers`.
- Re-target both implementations to `provider::Provider`.
- Keep the legacy `brain-providers` local modules in place for now.
- Add ignored, feature-gated smoke coverage for local-model inference in the new crates.

## Impact
- New standalone workspace crates and new public APIs.
- No `brain-*` integration or dependency in the new crates.
- Workspace docs/specs need to reflect the added crates.
