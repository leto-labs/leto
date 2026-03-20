# Tasks: add-nori-session-config-workspace

- [x] Add the Nori fork as a git submodule at `submodules/nori-cli`
- [x] Create the `feat/add-session-config` branch in the Nori fork workspace
- [x] Add an OpenSpec change that scopes ACP session config work to the Nori
      client instead of a `brain-acp` bridge
- [x] Define the initial Nori ACP scope as generic select-style session config
      options with a dedicated session-settings surface
- [ ] Implement generic ACP session config state in the Nori ACP connection
      layer
- [ ] Implement `session/set_config_option` and config update handling in the
      Nori ACP connection layer
- [ ] Add a dedicated ACP session-settings command and picker in the Nori TUI
- [ ] Add tests for ACP session config state, slash-command wiring, and picker
      behavior in the Nori fork
- [ ] Decide later whether ACP session modes need a separate follow-up branch
