# Tasks: add-opencode-compat-surface

## Planning Checklist

- [ ] Document the OpenCode compatibility direction as a secondary API layered
      on Brain's canonical runtime boundary
- [ ] Record `anomalyco/opencode` as the intended upstream compatibility target
      and reference consumer
- [ ] Define the compatibility scope as the session-oriented app API and record
      the non-goals for unrelated upstream route families
- [ ] Add explicit hard blockers covering remote runtime parity, store cleanup,
      hosted runtime/server integration, endpoint mapping, and
      auth/approval/workspace decisions
- [ ] Add a frontend-minimization rule that allows UI changes only for hard
      blockers such as CORS, auth transport, or platform networking
- [ ] Record that future upstream UI adoption may use a submodule or similar
      workflow, but defer that decision to a later implementation change
- [ ] Validate the change with
      `openspec validate add-opencode-compat-surface --strict`
