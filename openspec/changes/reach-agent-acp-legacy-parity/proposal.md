# Proposal: reach-agent-acp-legacy-parity

## Why

The remaining code-backed legacy migration gap is ACP, not server compatibility
or MCP.

Repo code still shows several behaviors that only exist in `brain-acp` today:

- standalone ACP binary identities for real and mock backends
- ACP client-owned file read/write bridging used by containerized clients
- ACP session-model mutation behavior currently consumed by Harbor
- repo tooling and launchers that still target `brain-acp`

Until those behaviors are either migrated or explicitly retired, `brain-acp`
cannot be deleted cleanly.

## What Changes

This change adds the ACP-specific parity requirements needed to finish the
legacy migration story:

1. direct `agent-acp` real-binary support plus an explicit decision on the mock
   backend path
2. real ACP backend support for client-owned file read/write bridging
3. real ACP session-model compatibility for the current Harbor integration
4. repo-tooling migration away from `brain-acp` launch identities

This change is intentionally limited to behaviors that exist in repo code
today. It does not treat OpenCode compatibility or MCP as ACP migration work.

## Impact

- Modified capability: `agent-acp`
- Modified capability: `repo-tooling`
- Defines the remaining active legacy-parity lane that is directly backed by
  code on disk
