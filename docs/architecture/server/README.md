# Server Surfaces

`agent-server` is the current hosted server surface over `AgentCore`.

## What It Contains

| Component | Role |
| --- | --- |
| `AgentCore` hosting | Shared app-facing core used by the server |
| Canonical `/v1` API | Hosted project/session/turn/credential/model surface |
| OpenCode compatibility | Secondary compat surface under `/v1/compat/opencode` |
| SSE and streaming | Runtime bus and turn-stream delivery |

## Current Status

| Question | Answer |
| --- | --- |
| Is `agent-server` the current hosted surface? | Yes |
| Do CLI and ACP depend on the same shared boundary? | Yes; all current product surfaces build on `AgentCore` |
## Architectural Reading

Treat `agent-server` as the live hosted boundary.
