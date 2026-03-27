## Overview

This change archives the completed credential architecture as its own focused
spec tranche.

The ownership split is:

- `provider` owns the standalone credential module, in-memory `CredentialPool`,
  and selection strategies
- `provider-openai` owns the standalone OAuth browser/device helpers, refresh
  helpers, and pool-backed provider adapters
- `agent-store` owns the persisted credential metadata required for OAuth
  refresh and health tracking
- `agent-core` owns activation of stored credentials into the shared runtime
  pool, including pre-turn refresh and post-turn health persistence

## Key Decisions

- The shared pool stays in `provider`, not `agent-core`, so provider crates
  remain standalone and reusable without app-runtime dependencies.
- `agent-core` does not own generic selection semantics. It owns the
  store-backed activation layer that materializes a chosen persisted subset into
  the shared pool.
- `provider-openai` no longer exposes an OpenAI-specific credential resolver
  contract. Managed credentials flow through the shared provider pool instead.

## Archive Strategy

This focused change is intended to be archived immediately after validation so
the canonical specs reflect the implemented credential architecture without
waiting for the rest of the parity umbrella to finish.
