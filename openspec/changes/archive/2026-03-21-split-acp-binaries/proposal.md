# Proposal: split-acp-binaries

## Why

The current mock ACP surface lives in the dedicated `brain-acp` crate, but the
main user-visible launch path is still `brain-cli -- acp`.

That creates two problems:

1. it makes `brain-cli` look like the owner of ACP when it is only a thin
   forwarder to `brain-acp`
2. it leaves no clean product boundary between the current mock ACP runtime and
   the future real ACP runtime that should also live in `brain-acp`

The project now wants a clearer split:

- `brain-acp` should own the ACP binary surface directly
- a mock ACP runtime should remain available explicitly for testing and TUI
  iteration
- external clients such as Nori should be able to register both identities now
  so the future real cutover does not require another config-shape change

## What

This change will:

- add two binaries to the `brain-acp` crate:
  - `brain-acp`
  - `brain-acp-mock`
- keep both binaries temporarily backed by the current mock ACP runtime
- keep `brain-cli -- acp` only as a compatibility alias
- keep the repo-owned `scripts/brain-acp-launcher.sh` and retarget it to the
  explicit mock binary for `acpx` compatibility testing
- update local Nori configuration to register both ACP agents side by side

## Impact

- Modified capability: `brain-acp`
- Packaging change: ACP becomes directly owned by `brain-acp`
- Client integration change: Nori gets separate `brain-acp` and
  `brain-acp-mock` entries
- Compatibility behavior: `brain-cli -- acp` remains temporarily supported but
  is no longer the canonical ACP launch path
