- [x] Add a new OpenSpec change for `provider-anthropic`
- [x] Add the `provider-anthropic` workspace crate with standalone config,
      client, error, and Messages types
- [x] Add Anthropic Messages create and SSE stream support
- [x] Add unit tests for serialization and parser behavior
- [x] Add env-gated live smoke tests using `ANTHROPIC_API_KEY`
- [x] Add `@anthropic-ai/sdk` to `repocache`
- [x] Validate with `openspec validate add-provider-anthropic --strict`
- [x] Run `cargo test -p provider-anthropic` and `cargo test --workspace`
