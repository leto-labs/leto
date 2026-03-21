# Design: split-acp-binaries

## Decision: Separate Binaries, Not A Runtime Backend Flag

The ACP surface should use two binaries rather than a `--backend` selector.

That is the cleaner product model:

- `brain-acp` is the long-term real ACP binary
- `brain-acp-mock` is the explicit mock and development ACP binary

This avoids shipping a mock toggle as part of the long-term product-facing
binary contract.

## Decision: Shared Library Runtime Helpers

The `brain-acp` crate should keep the stdio wiring in shared library code and
let each binary call a small runtime entrypoint.

For this change, both binaries may call the same mock runtime helper. That is a
temporary implementation detail, not the long-term product contract.

## Decision: Keep The `acpx` Launcher Script

The repo-owned launcher script remains useful because the `acpx` compatibility
harness launches the agent from a temporary working directory.

The script should continue to:

- `cd` to the repo root
- execute the explicit mock binary

This keeps Cargo workspace resolution stable for external-client testing.

## Decision: Dual Nori Agent Registration

The local Nori configuration should register two agents immediately:

- `brain-acp`
- `brain-acp-mock`

Both entries will currently be mock-backed. The recommended default selected
agent remains the explicit mock entry so current semantics stay obvious.

Later, only the `brain-acp` binary implementation changes to the real backend.
The config shape and agent identities stay stable.
