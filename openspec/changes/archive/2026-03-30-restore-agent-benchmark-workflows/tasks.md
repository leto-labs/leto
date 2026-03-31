# Tasks: restore-agent-benchmark-workflows

- [x] Restore repo-local Harbor `agent` and `agent-acp` helper surfaces on the
      live stack
- [x] Restore the `acpx` compatibility harness through an explicit
      `agent-acp-mock` launch path
- [x] Add OpenSpec deltas for the migrated benchmark and ACP launch surfaces
- [x] Validate the change with
      `openspec validate restore-agent-benchmark-workflows --strict`
