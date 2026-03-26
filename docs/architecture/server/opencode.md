# OpenCode Compatibility

This note documents the preferred way to work on OpenCode compatibility in
`agent-server`.

The important example is the permission route family in
`crates/agent-server/src/compat/opencode/routes/permission.rs`. That file shows
the shape we want going forward:

- route-local OpenAPI metadata declared next to the Axum handlers
- direct `axum + aide` usage instead of compat-specific request/response schema
  builders
- small, generic test helpers that normalize representation noise without
  hiding real schema mismatches

The rest of the older compat stack still contains broader normalization and
manual schema shaping. Treat that as migration debt, not as the pattern to copy
for new work.

## Preferred Shape

The permission routes use `aide::axum::routing::get_with` and `post_with`
directly and attach only route-local metadata that belongs in the operation
itself:

- `operationId`
- summary
- description
- explicit response metadata when the runtime return type is too opaque for
  `aide` to infer cleanly

This keeps the documentation surface close to the runtime surface. The route,
handler extractors, and OpenAPI operation transform all live in one place.

That is the right default for compat work.

## What To Avoid

Avoid treating the entire compat document as the unit of change when the issue
is route-local.

In practice, that means avoiding these patterns unless there is no better
option:

- hand-building request or response schemas with raw JSON when direct
  `aide` route metadata can express the contract
- route helpers that bypass `aide`'s normal input/output/schema registry flow
- large compat-specific post-processing layers used to force a full pinned-doc
  match before we understand the route-level mismatch
- DTO-level `schema_with` overrides for representation-only differences that can
  be handled more clearly in test normalization

Some of those patterns already exist in the older compat stack. They are not
the recommended model for new work.

## Route-Level Workflow

When a route does not match `openapi/opencode.json`, debug it in isolation.

The permission route tests do that by:

1. building a one-route `ApiRouter`
2. generating a small OpenAPI doc from that router with `generate_from_router`
3. extracting the matching path subset from the pinned `openapi/opencode.json`
4. comparing the two after a small amount of generic normalization

The key detail is that `generate_from_router` takes a router builder closure,
not a pre-built router. `aide` records schema information while the router is
being constructed, so the generator context must be configured before route
construction begins.

This is why the helper in `crates/agent-server/src/utils/openapi.rs` is shaped
as:

- configure `aide::generate`
- build the router
- call `finish_api_with`

and not the other way around.

## Fix Strategy

The permission route work split mismatches into two categories:

- source-level schema/modeling issues
- test-level representation noise

That distinction matters. We should fix real schema problems in the DTOs and
route definitions. We should only normalize comparison noise in the test
harness.

## Source-Level Fixes

These are the fixes that belonged in the source model rather than in test
helpers.

### Keep Optional Fields Optional, Not Nullable

`schemars` encodes `Option<T>` as nullable by default. Upstream OpenCode often
models the same field as optional-but-not-nullable.

For fields where the pinned contract expects an optional field with a non-null
schema, the fix is:

- keep the Rust field as `Option<T>`
- keep `#[serde(default, skip_serializing_if = "Option::is_none")]`
- add `#[schemars(with = "...")]` so the schema is derived from `T` rather than
  `Option<T>`

Examples from the permission DTOs:

- `PermissionRequestDoc.tool` uses `#[schemars(with = "ToolRequestDoc")]`
- `PermissionReplyRequest.message` uses `#[schemars(with = "String")]`

### Inline Small Nested Types at the Type Level

For small nested helper DTOs that should inline where they are used, prefer
type-level `#[schemars(inline)]`.

This worked for the permission route where field-level `inline` support was not
available in the derive parser we are actually using.

Examples:

- `ToolRequestDoc`
- `PermissionReplyRequest`
- `PermissionReplyValueDoc`
- `NotFoundDataDoc`
- `NotFoundErrorNameDoc`

This keeps the source readable and avoids custom schema functions for cases that
are fundamentally just ref-vs-inline representation choices.

### Model Maps Using a Schema Shape That Matches the Contract

When the pinned contract wants an object/map shape, prefer an explicit
`schemars(with = "...")` mapping instead of fighting the emitted schema later.

Example:

- `PermissionRequestDoc.metadata` uses
  `#[schemars(with = "std::collections::BTreeMap<String, Value>")]`

That gave a more contract-shaped schema than the previous looser form.

## Test-Level Normalization

These normalizations live in
`crates/agent-server/src/utils/openapi.rs`. They are intentionally generic and
should stay generic.

They exist to remove low-value representation differences so the remaining diff
shows the real mismatch.

### Exclude Known Extra Responses

`exclude_response_codes` removes known extra `aide` responses such as `415` and
`422` when those codes are not part of the pinned OpenCode contract for the
focused route comparison.

### Prune Unreferenced Schemas

`exclude_unreferenced_schemas` removes collected component schemas that are not
actually referenced by the final route fragment.

This was important for types like `CompatQuery`, where the route parameters were
already inlined and the leftover component was just generator inventory.

### Drop Parameter Representation Noise

`exclude_parameter_fields` removes selected relative fields from parameter
objects.

The permission tests currently use it for:

- `style`
- `schema/default`

This keeps the comparison focused on parameter meaning rather than parameter
verbosity.

### Drop Request Body Representation Noise

`exclude_request_body_fields` removes selected relative fields from request body
objects.

The permission tests currently use it for:

- `required`

This handles OpenAPI metadata noise without changing request body schemas
themselves.

### Drop Schema Metadata Noise

`exclude_schema_metadata_fields` removes selected relative metadata fields from
each discovered schema node root.

The permission tests currently use it for:

- `additionalProperties`
- `propertyNames`

This is intentionally node-scoped. It is not a broad delete-by-key sweep across
the whole document.

### Normalize Single-Value Enums to `const`

`normalize_single_value_enums_to_const` rewrites schema nodes of the form:

```json
{ "type": "string", "enum": ["X"] }
```

to:

```json
{ "type": "string", "const": "X" }
```

This is a good example of comparison-layer normalization. The Rust DTO stays a
normal one-variant enum, and the test helper handles the representation choice.

### Normalize Schema-Node `true` to `{}`

`normalize_true_schemas_to_empty_objects` rewrites schema nodes equal to the
boolean schema `true` into `{}`.

This was the final remaining permission reply mismatch:

- generated `BadRequestError.data` came from `serde_json::Value`, which
  `schemars` emits as `true`
- pinned OpenCode uses `{}` for the same unrestricted schema

The helper is scoped to schema nodes only. It does not rewrite arbitrary
booleans elsewhere in the OpenAPI document.

## The Permission Route as the Reference Example

The permission route family is the current best reference for how to work on
OpenCode compatibility:

- use direct `aide` route definitions in
  `crates/agent-server/src/compat/opencode/routes/permission.rs`
- keep DTO fixes close to the DTOs in
  `crates/agent-server/src/compat/opencode/types/permission.rs` and
  `crates/agent-server/src/compat/opencode/types/errors.rs`
- use small, generic comparison helpers in
  `crates/agent-server/src/utils/openapi.rs`
- prefer route-level parity tests over broad whole-document patching while
  debugging a specific mismatch

That is the "right" way to evolve this compatibility layer. The older
compat-wide schema injection and normalization logic may still exist, but it
should be treated as legacy debt to reduce over time, not as the design target.
