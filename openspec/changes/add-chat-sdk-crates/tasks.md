- [x] Add OpenSpec change files and capability deltas for `chat`,
      `chat-telegram`, `chat-slack`, and `chat-teams`
- [x] Add the standalone `chat` workspace crate with shared traits, types,
      errors, docs, and tests
- [x] Add the `chat-telegram` workspace crate with config, adapter, and
      normalization tests
- [x] Add the `chat-slack` workspace crate with config, adapter, and
      normalization tests
- [x] Add the `chat-teams` workspace crate with config, adapter, and
      normalization tests
- [x] Wire the new crates into the workspace manifest and workspace dependencies
- [x] Validate with `openspec validate add-chat-sdk-crates --strict`
- [x] Run `cargo test -p chat -p chat-telegram -p chat-slack -p chat-teams`
- [x] Run `cargo test --workspace`
