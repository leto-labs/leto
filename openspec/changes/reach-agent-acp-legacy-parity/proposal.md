# Proposal: reach-agent-acp-legacy-parity

## Why

The remaining code-backed legacy migration gap is ACP, not server compatibility
or MCP.

Repo code still shows one behavior that only exists in the legacy ACP
implementation today:

- ACP client-owned file read/write bridging used by external ACP clients

The benchmark runner and direct ACP binary identities have migrated to the live
stack, and Harbor validation now passes on the live backend through native task
workspace access. The one remaining migration gap is still the client-owned
filesystem bridge itself.

## What Changes

This change now tracks the remaining ACP-specific parity requirement needed to
finish the legacy migration story:

1. real ACP backend support for client-owned file read/write bridging

This change is intentionally limited to behaviors that exist in repo code
today. It does not treat OpenCode compatibility or MCP as ACP migration work.

## Impact

- Modified capability: `agent-acp`
- Defines the remaining active legacy-parity lane that is still directly backed
  by code on disk after Harbor/runtime stabilization
