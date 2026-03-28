# agent-server modularization plan

This note captures the `agent-server` architecture hotspots that were large
enough to slow down navigation and make route-family ownership unclear.

## Baseline hotspots

Measured before the first extraction in this change:

| Lines | File | Current responsibility | Proposed split |
| ---: | --- | --- | --- |
| 3629 | `src/utils/openapi.rs` | Generic OpenAPI subsetting, schema walking, normalization, and expansion helpers | Split into `utils/openapi/{subset,walk,normalize,expand}.rs` so generator normalization is isolated from document slicing |
| 1576 | `src/compat/opencode/routes/session.rs` | Session route registration plus route-local schema shaping helpers and all session handlers | Split into `routes/session/{listing,lifecycle,messages,prompts,parts,docs}.rs` with a thin `mod.rs` |
| 1384 | `src/http.rs` | Canonical router, canonical handlers, chat completion adapter, turn streaming, credential CRUD, and error mapping | Completed across two steps by extracting `http/errors.rs` and `http/routes/{system,projects,providers,sessions,chat_completions,turns,credentials}.rs` |
| 1195 | `src/compat/opencode/types/session.rs` | Session requests, query/path DTOs, message docs, part docs, status docs, and schema helper enums | Split into `types/session/{requests,queries,paths,messages,parts,status}.rs` |
| 1117 | `tests/http_integration.rs` | Canonical API tests, compat tests, SSE checks, doc checks, and harness helpers | Split into `tests/http/{canonical_core,canonical_turns,canonical_chat,compat_surface}.rs` plus shared helpers |
| 997 | `src/compat/opencode/mod.rs` | Compat state lookups, DTO conversions, prompt orchestration, auth helpers, path helpers, and misc utilities | Split into `compat/opencode/{convert,prompts,auth,fs,ids}.rs` and keep `mod.rs` as the integration point |

## Canonical HTTP completed here

The canonical HTTP surface was the safest place to start because it already had
clear concern boundaries and strong integration coverage. This change extracts:

- `src/http/errors.rs`
- `src/http/routes/system.rs`
- `src/http/routes/projects.rs`
- `src/http/routes/providers.rs`
- `src/http/routes/sessions.rs`
- `src/http/routes/chat_completions.rs`
- `src/http/routes/turns.rs`
- `src/http/routes/credentials.rs`

That reduces `src/http.rs` from 1384 lines in the baseline inventory to 184
lines while keeping route registration in one place.

## Recommended next steps

1. Split `compat/opencode/mod.rs` into conversion, prompt execution, auth, and
   identifier/path helper modules. That file is acting as the shared utility
   sink for the entire compat surface.
2. Split `compat/opencode/routes/session.rs` by route cluster. The file mixes
   lifecycle routes, prompt routes, message routes, and a large amount of
   route-local OpenAPI schema customization.
3. Split `compat/opencode/types/session.rs` by schema family so request DTOs,
   message docs, part docs, and status docs stop changing together.
4. Split `tests/http_integration.rs` by API family. Right now canonical and
   compat regressions land in one file, which hides ownership and slows review.
5. Isolate OpenAPI normalization passes in `utils/openapi.rs`. That file is
   generic enough to stay independent, but it should not require scrolling
   across four different transformation concerns to change one pass.

## Heuristic used

The trigger here was not line count by itself. Each file above currently mixes
multiple concerns that change for different reasons:

- handler registration and business logic
- DTO declarations and schema documentation
- compatibility conversion and prompt execution
- generic OpenAPI transforms and test-only helpers

The split plan keeps each module aligned with one route family or one document
transformation concern so ownership stays obvious and future changes stay local.
