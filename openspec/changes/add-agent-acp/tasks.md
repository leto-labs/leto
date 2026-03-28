- [ ] Add OpenSpec change files for the new `agent-acp` capability and
      migration plan
- [ ] Add the `agent-acp` workspace crate and wire it into the workspace
      manifest
- [ ] Implement an ACP backend adapter that wraps `AgentCore` rather than
      `BrainRuntime`
- [ ] Support ACP session creation, loading, listing, prompt execution, and
      cancellation through `AgentCore`
- [ ] Support ACP session config options for model, thought level, and loop
      through persisted `agent-store` session state
- [ ] Replay stored history and map streamed `AgentCore` turn events into ACP
      session updates
- [ ] Add tests covering ACP config options, event mapping, and history replay
- [ ] Validate the OpenSpec change with `openspec validate add-agent-acp --strict`
