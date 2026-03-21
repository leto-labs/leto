# Tasks: split-acp-binaries

## Spec

- [x] Add OpenSpec proposal, design, tasks, and `brain-acp` spec delta
- [x] Validate the change with `openspec validate split-acp-binaries --strict`

## Implementation

- [x] Add `brain-acp` and `brain-acp-mock` binaries to the `brain-acp` crate
- [x] Refactor `brain-acp` library exports around a shared mock stdio runtime
- [x] Keep `brain-cli -- acp` as a compatibility alias
- [x] Retarget `scripts/brain-acp-launcher.sh` to the explicit mock binary
- [x] Update the `acpx` compatibility harness to treat the launcher script as the mock path
- [x] Update the local Nori config to register both ACP agents

## Verification

- [x] Update ACP docs to describe the binary split and dual Nori registration
- [x] Run `cargo test --workspace`
- [x] Confirm `openspec validate split-acp-binaries --strict` passes
