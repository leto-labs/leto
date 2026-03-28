# Proposal: refactor-agent-server-modularization

## Why

`agent-server` has several architecture hotspots where one file owns multiple
unrelated concerns:

- canonical HTTP routing, transport adaptation, and error translation
- OpenCode compat session route registration and route-local schema shaping
- OpenCode compat session DTO families
- generic OpenAPI document traversal, normalization, and expansion
- canonical and compat integration coverage in one test file

That makes routine work slower because a small change requires reloading a large
mixed-responsibility file, and it weakens ownership boundaries between route
families.

## What

This change starts modularization in the canonical HTTP surface and records the
next split boundaries for the remaining hotspots.

Phase 1 implemented in this change:

- extract canonical chat completion handling into `src/http/chat_completions.rs`
- extract canonical turn endpoints into `src/http/turns.rs`
- extract canonical credential endpoints into `src/http/credentials.rs`
- extract shared canonical HTTP error translation into `src/http/errors.rs`
- add a crate-local modularization note documenting the remaining >800-line
  files and recommended module boundaries

Follow-on work proposed by this change:

- split OpenCode compat session routes by route cluster
- split OpenCode compat session DTOs by schema family
- split compat shared helpers in `compat/opencode/mod.rs` by concern
- split the large generic OpenAPI utility file by transformation pass
- split HTTP integration coverage by API family

## Impact

- No intended behavior change for the canonical HTTP API
- Lower navigation cost in `agent-server`
- Clearer ownership for future compat and testing refactors
