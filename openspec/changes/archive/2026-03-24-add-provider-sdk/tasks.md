- [x] Add a new OpenSpec change for the shared `provider` SDK crate
- [x] Add the `provider` workspace crate with shared trait, request types,
      event model, capabilities, errors, and mock implementation
- [x] Add a shared provider implementation to `provider-openai`
- [x] Add a shared provider implementation to `provider-anthropic`
- [x] Add unit tests for the shared request and event model
- [x] Add adapter tests and env-gated smoke tests for the shared provider
      implementations
- [x] Validate with `openspec validate add-provider-sdk --strict`
- [x] Run `cargo test -p provider`, `cargo test -p provider-openai`,
      `cargo test -p provider-anthropic`, and `cargo test --workspace`
