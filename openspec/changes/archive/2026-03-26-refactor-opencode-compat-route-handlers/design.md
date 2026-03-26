# Design: refactor-opencode-compat-route-handlers

## Decisions

- Route-family files under `compat/opencode/routes/` own all HTTP-facing Axum
  handlers for their family.
- `compat/opencode/mod.rs` remains the home for shared state aliases,
  conversion helpers, prompt/message mutation helpers, compat ID parsing, and
  other cross-family internals.
- Shared non-HTTP helpers remain centralized until a stronger internal service
  split is justified. This keeps the refactor focused on HTTP-boundary
  ownership rather than broader architectural churn.

## Non-Goals

- No changes to compat path or schema behavior
- No change to canonical `/v1`
- No changes to `/v1/compat/opencode/doc` beyond preserving exact parity
