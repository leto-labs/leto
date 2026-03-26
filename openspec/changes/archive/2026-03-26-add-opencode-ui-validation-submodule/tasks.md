# Tasks: add-opencode-ui-validation-submodule

## Spec

- [x] Add a `repo-tooling` delta for a pinned OpenCode UI validation submodule
- [x] Record that the submodule remote uses the synced `leto-labs/opencode`
      fork but stays pinned to the release-aligned commit used by compat work
- [x] Record that local UI edits must stay minimal and blocker-driven

## Implementation

- [x] Add `submodules/opencode` as a Git submodule
- [x] Pin the submodule to the fork commit equivalent to upstream OpenCode
      `v1.3.2`
- [x] Update `repocache/repocache.json` so the OpenCode entry matches the
      approved `v1.3.2` compatibility pin
- [x] Document the local workflow for running the pinned OpenCode UI against
      `agent-server`
- [x] Smoke-test the pinned OpenCode UI against local `agent-server`
- [x] If a validation blocker appears, keep any fork changes minimal and
      document the reason

## Verification

- [x] Validate the change with
      `openspec validate add-opencode-ui-validation-submodule --strict`
