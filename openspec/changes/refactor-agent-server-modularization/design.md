# Design: refactor-agent-server-modularization

## Context

The baseline `agent-server` inventory showed six files above 800 lines:

| Lines | File |
| ---: | --- |
| 3629 | `crates/agent-server/src/utils/openapi.rs` |
| 1576 | `crates/agent-server/src/compat/opencode/routes/session.rs` |
| 1384 | `crates/agent-server/src/http.rs` |
| 1195 | `crates/agent-server/src/compat/opencode/types/session.rs` |
| 1117 | `crates/agent-server/tests/http_integration.rs` |
| 997 | `crates/agent-server/src/compat/opencode/mod.rs` |

The issue is not the line count alone. Each file combines concerns that change
for different reasons.

## Decision

Start with the canonical HTTP layer because it has the clearest split points
and the lowest behavior risk:

- chat completion adaptation is self-contained
- turn transport endpoints are self-contained
- credential CRUD is self-contained
- HTTP error translation is reusable shared infrastructure

Keep route registration in `src/http.rs` so the canonical route inventory stays
easy to scan in one place, then move the remaining project, provider/model,
system-event, and session/runtime handlers into route-family modules under
`src/http/routes/`.

## Module boundaries

### Canonical HTTP

- `src/http.rs`
  - router construction
  - route inventory
  - ID parsing and small shared helpers
- `src/http/routes/system.rs`
  - health, status, agent inventory, and event streaming
- `src/http/routes/projects.rs`
  - project CRUD
  - project-root lookup and resolution
  - project session creation/listing
- `src/http/routes/providers.rs`
  - provider catalog and model lookup
- `src/http/routes/sessions.rs`
  - session CRUD
  - messages, trajectories, and runtime view
- `src/http/routes/chat_completions.rs`
  - OpenAI-compatible chat completion request adaptation
  - temporary session lifecycle used by `/v1/chat/completions`
- `src/http/routes/turns.rs`
  - NDJSON and SSE turn transport handlers
  - tool-call append adapter
- `src/http/routes/credentials.rs`
  - credential CRUD and health projections
- `src/http/errors.rs`
  - `CoreError` and `StoreError` to HTTP response mapping

### Next compat split

- `src/compat/opencode/mod.rs`
  - move DTO conversion into `convert.rs`
  - move prompt execution into `prompts.rs`
  - move auth and credential glue into `auth.rs`
  - move path and ID helpers into `ids.rs` and `fs.rs`
- `src/compat/opencode/routes/session.rs`
  - move route builders into route-cluster modules:
    `listing.rs`, `lifecycle.rs`, `messages.rs`, `prompts.rs`, `parts.rs`
  - move route-local OpenAPI schema utilities into `docs.rs`
- `src/compat/opencode/types/session.rs`
  - move request/query/path DTOs away from response and part schemas

### Test split

- `tests/http_integration.rs`
  - keep harness helpers in `tests/http/support.rs`
  - split canonical API, turns/chat, and compat assertions into separate files

## Trade-offs

Pros:

- Smaller edit surfaces
- Better review locality
- Clearer ownership by route family

Cons:

- More files to navigate
- Slightly more module plumbing

This trade-off is worth it because the current files already span several
independent concerns.
