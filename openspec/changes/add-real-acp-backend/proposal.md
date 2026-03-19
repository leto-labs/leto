# Proposal: add-real-acp-backend

## Why

`brain-acp` currently proves ACP interoperability through a large standalone
mock implementation, but it does not exercise the actual `brain-core` runtime.

That is now the main gap:

- ACP sessions are not backed by real `brain` sessions
- ACP prompt turns are not backed by `Brain::turn()`
- project resolution is not shared with the core store/runtime model
- the mock implementation is hard to compare against a future real backend

The project now wants a production-oriented ACP backend that:

- uses `brain-core` as the execution substrate
- uses the modular store/provider/loop/tool stack already defined in the
  workspace
- keeps ACP protocol types isolated to `brain-acp`
- preserves a mirrored mock backend for side-by-side comparison during
  development and debugging

## What

This change will:

- refactor `brain-acp` into mirrored `backend/` and `mock/` module trees
- keep `brain-acp-mock` as the explicit mock binary
- make `brain-acp` use a real `brain-core::Brain` stack for session create/load/list,
  prompt streaming, and cancellation
- add store/core support for resolving projects by working directory root
- keep ACP SDK types as the protocol source of truth inside `brain-acp`
- defer ACP client-owned requests and richer ACP parity to follow-up changes

## Impact

- Modified capability: `brain-acp`
- Modified capability: `brain-core`
- Modified capability: `brain-types`
- Modified capability: `brain-stores`
