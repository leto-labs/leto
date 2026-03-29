- [x] Add OpenSpec change files for the new `agent-acp` capability and
      migration plan
- [x] Add the `agent-acp` workspace crate and wire it into the workspace
      manifest
- [x] Implement an ACP backend adapter that wraps `AgentCore` rather than
      `BrainRuntime`
- [x] Support ACP session creation, loading, listing, prompt execution, and
      cancellation through `AgentCore`
- [x] Support ACP session config options for model, thought level, and loop
      through persisted `agent-store` session state
- [x] Replay stored history and map streamed `AgentCore` turn events into ACP
      session updates
- [x] Add tests covering ACP config options, event mapping, and history replay
- [x] Validate the OpenSpec change with `openspec validate add-agent-acp --strict`
