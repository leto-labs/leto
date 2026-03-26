# Proposal: eliminate-opencode-doc-only-dtos

## Why

The OpenCode compat subtree had drifted into a split contract model:

- OpenAPI generation used types from a `docs` module
- multiple handlers still parsed looser runtime inputs such as
  `Query<BTreeMap<String, String>>`
- several compat adapters still emitted `serde_json::Value` instead of the same
  DTOs the generated spec described

That made the compat contract harder to reason about and weakened the claim
that the generated `/v1/compat/opencode/doc` document is the same contract the
runtime actually implements.

## What

This change turns the compat contract types into shared runtime-and-doc DTOs.

It:

- removes the docs-only `types::docs` framing and replaces it with a shared
  contract module
- updates compat handlers to use the same typed query/body/response DTOs that
  drive OpenAPI generation
- converts key compat state and response adapters away from raw `Value` blobs
  into contract DTOs
- keeps exact prefix-aware `/doc` parity green throughout

## Impact

- Modified capability: `agent-server`
- No intended canonical `/v1` contract changes
- No intended OpenCode compat wire-contract changes
- Stronger guarantee that the generated compat spec and runtime handlers share a
  single contract model
