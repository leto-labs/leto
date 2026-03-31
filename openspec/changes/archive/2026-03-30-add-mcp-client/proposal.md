# Proposal: add-mcp-client

## Why

This proposal no longer matches the committed architecture.

It was written against legacy `brain-tools`, but that crate is being removed
from the committed tree and preserved only in the local `archive/brain/`
reference archive. Keeping this change active would imply future work on a
capability that is no longer part of the live workspace.

## What
This change is being retired rather than implemented.

The architectural decision is:

- do not keep MCP work scoped to archived legacy `brain-*` crates
- revisit MCP later only on the live `agent-*` tool/runtime stack
- preserve this proposal only as historical context for an abandoned
  legacy-scoped plan

## Impact

- No implementation change: this proposal is archived as superseded
- Removes a legacy-targeted MCP plan from the active queue
