# Tasks: reach-agent-acp-legacy-parity

## ACP Parity Work

- [ ] Add `agent-acp` requirements for direct real ACP binary support and an
      explicit real/mock launch strategy
- [ ] Add `agent-acp` requirements for ACP client-owned file read/write
      bridging
- [ ] Add `agent-acp` requirements for session-model compatibility needed by
      current Harbor usage
- [ ] Add `repo-tooling` requirements for Harbor and repo-owned ACP launchers
      to stop depending on `brain-acp`
- [ ] Validate the change with
      `openspec validate reach-agent-acp-legacy-parity --strict`
