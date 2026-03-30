# Transports

The modern stack keeps transport concerns intentionally thin. Local human-facing
I/O mostly lives in `agent-cli`, ACP transport mapping lives in `agent-acp`,
and hosted delivery lives in `agent-server`. The older `brain-transports`
crate remains as legacy transport infrastructure rather than the primary path.

## Current Surface

| Type | Role |
| --- | --- |
| CLI stdin/stdout loop | handled directly inside `agent-cli` |
| ACP stdio adapter | handled by `agent-acp` |
| HTTP/SSE transport | handled by `agent-server` |

## Relationship To Other Surfaces

| Surface | Where it belongs |
| --- | --- |
| Interactive local CLI | `agent-cli` |
| Live session engine | `agent-runtime` |
| Application/core boundary | `agent-core` |
| ACP client/server mapping | `agent-acp` |
| Hosted remote APIs | `agent-server` |

## Architectural Reading

Transport stays thin because richer UX concerns now sit above the session
engine in `agent-core`, `agent-cli`, `agent-acp`, and `agent-server` rather
than inside a dedicated transport crate.
