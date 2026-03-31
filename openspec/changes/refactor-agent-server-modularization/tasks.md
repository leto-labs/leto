# Tasks

- [x] Inventory oversized `agent-server` files and define split boundaries
- [x] Add a crate-local modularization note with the >800-line inventory and
  proposed module map
- [x] Extract canonical HTTP chat completion handling into a dedicated module
- [x] Extract canonical HTTP turn handling into a dedicated module
- [x] Extract canonical HTTP credential handling into a dedicated module
- [x] Extract canonical HTTP error translation into a dedicated module
- [x] Split remaining canonical HTTP handlers into `src/http/routes/*.rs`
  modules by endpoint family while keeping route registration centralized
- [ ] Split `compat/opencode/mod.rs` by conversion, prompt, auth, and path
  concerns
- [x] Split `compat/opencode/routes/session.rs` by route cluster and route-doc
  utilities
- [x] Split `compat/opencode/mod.rs` by conversion, prompt, auth, and path
  concerns
- [x] Split `compat/opencode/types/session.rs` by schema family
- [x] Split `tests/http_integration.rs` by API family
