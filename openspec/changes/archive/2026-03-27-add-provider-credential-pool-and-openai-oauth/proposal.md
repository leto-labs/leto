## Why

The provider-credential tranche is already implemented, but it currently lives
inside the broader `reach-agent-stack-legacy-parity` umbrella change.

That makes the canonical specs stale even though the shared credential pool,
the standalone OpenAI OAuth surface, the richer store credential metadata, and
the `agent-core` activation layer are already working.

## What Changes

- add canonical spec coverage for the shared provider credential module and
  selection strategies
- add canonical spec coverage for the pool-backed and OAuth-backed
  `provider-openai` surfaces
- add canonical spec coverage for the richer `agent-store` credential model
- add canonical spec coverage for `agent-core` activation of stored
  credentials into the shared provider pool

## Impact

- updates the canonical `provider`, `provider-openai`, `agent-store`, and
  `agent-core` specs to match the shipped credential architecture
- leaves the broader parity umbrella open only for the still-unfinished
  bootstrap, loop, tool, and server work
