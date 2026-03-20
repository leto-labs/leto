# Tasks: add-real-acp-config-options

- [x] Add an OpenSpec change for real ACP config options on `brain-acp`
- [x] Extend shared inference/session types to persist a thought-level override
- [x] Update core runtime logic to merge, validate, and persist thought-level overrides
- [x] Add real backend helpers to build ACP `configOptions` for `model` and `thought_level`
- [x] Implement `configOptions` on `new_session` and `load_session`
- [x] Implement real `session/set_config_option`
- [x] Group model config options by provider in ACP responses
- [x] Add tests for shared inference merge behavior and real ACP config-option responses
- [x] Run `cargo test --workspace`
- [x] Validate `openspec validate add-real-acp-config-options --strict`
