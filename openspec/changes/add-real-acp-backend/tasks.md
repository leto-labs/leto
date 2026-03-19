# Tasks: add-real-acp-backend

- [x] Add an OpenSpec change for the real ACP backend and mirrored mock split
- [x] Refactor `brain-acp` into mirrored `backend/` and `mock/` module trees
- [x] Preserve existing mock ACP behavior through the new mock module layout
- [x] Add project lookup by root to the store contract and store implementations
- [x] Add `Brain::resolve_or_create_project(root)` to support ACP cwd mapping
- [x] Implement the real ACP backend on top of `brain-core::Brain`
- [x] Add real ACP session create/load/list/prompt/cancel coverage
- [x] Validate the OpenSpec change and run workspace tests
