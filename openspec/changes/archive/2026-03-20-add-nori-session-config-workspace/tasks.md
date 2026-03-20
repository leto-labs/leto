# Tasks: add-nori-session-config-workspace

- [x] Add the Nori fork as a git submodule at `submodules/nori-cli`
- [x] Create the `feat/add-session-config` branch in the Nori fork workspace
- [x] Add an OpenSpec change that scopes ACP session config work to the Nori
      client instead of a `brain-acp` bridge
- [x] Define the initial Nori ACP scope as generic select-style session config
      options with a dedicated session-settings surface
- [x] Implement generic ACP session config state in the Nori ACP connection
      layer
- [x] Implement `session/set_config_option` and config update handling in the
      Nori ACP connection layer
- [x] Add a dedicated ACP session-settings command and picker in the Nori TUI
- [x] Add tests for ACP session config state, slash-command wiring, and picker
      behavior in the Nori fork
- [x] Confirm ACP session modes remain out of scope for this branch
- [x] Record `just nori` / `cargo run --bin nori --` as the primary local dev
      loop for the fork and keep packaging artifacts out of the tracked
      workflow
