# Proposal: add-opencode-compat-surface

## Why

This planning change no longer matches the codebase or the migration problem it
was being used to describe.

The repo now has real `agent-server` compatibility work in flight, but legacy
`brain-server` never implemented an OpenCode compatibility layer. That makes
this change misleading in two ways:

1. it frames OpenCode compatibility as future `brain-server` work rather than
   current `agent-server` work
2. it incorrectly suggests that OpenCode compatibility is part of the legacy
   `brain-*` deletion baseline

OpenCode compatibility remains valid product work, but it should be tracked on
the current `agent-server` stack instead of through a deferred `BrainRuntime` /
`brain-server` planning change.

## What

This change is being retired rather than implemented.

The architectural decision is:

- keep OpenCode compatibility scoped to the current `agent-server` surface
- stop describing OpenCode compatibility as future `brain-server` work
- stop treating OpenCode compatibility as a blocker for deleting legacy
  `brain-*` crates
- preserve this change only as historical planning context for an abandoned
  legacy-scoped approach

## Impact

- No implementation change: this proposal is archived as superseded
- Clarifies direction: OpenCode compatibility is current `agent-server` work,
  not future `brain-server` work
- Removes a false legacy-migration blocker from the active OpenSpec queue
