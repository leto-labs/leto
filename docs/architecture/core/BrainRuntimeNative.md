# AgentCoreNative

`AgentCoreNative` is the current concrete implementation of the shared
`AgentCore` boundary. It is the local core used by `agent-cli`, `agent-acp`,
and `agent-server`.

## Responsibilities

| Area | Current behavior |
| --- | --- |
| Store ownership | Holds one shared `Store` instance |
| Registries | Owns provider, tool, and loop registries or equivalent named inventories |
| Defaults | Tracks default provider name and default loop name |
| Turn lifecycle | Rejects concurrent turns per session and owns cancellation tokens |
| Event bus | Publishes duplexed turn events and store lifecycle events |
| Session helpers | Resolves current/effective model and loop state from project + session config |

## Resolution Model

| Concern | Resolution order |
| --- | --- |
| Provider | explicit session/project provider -> unique model owner -> runtime default provider |
| Loop | session loop override -> project loop setting -> runtime default loop |
| Tools | all registered tools |

## Project Bootstrap

When a project root is first resolved, the runtime:

| Step | Behavior |
| --- | --- |
| 1 | normalizes the filesystem root |
| 2 | loads layered config through the `agent-core` bootstrap path |
| 3 | uses root `.agents/AGENTS.md` only when config did not already set a prompt |
| 4 | persists the created project through the store |

## Turn Path

```mermaid
sequenceDiagram
    participant Client
    participant Core as AgentCoreNative
    participant Registry as Registries
    participant Runtime as SessionEngine
    participant Store

    Client->>Core: turn(session_id, input)
    Core->>Core: reject duplicate active turn if needed
    Core->>Store: load session/project config
    Core->>Registry: resolve provider, loop, tools
    Core->>Runtime: run turn with resolved dependencies
    Runtime->>Store: persist messages and trajectory
    Core-->>Client: event stream
```

## Why It Is Important

`AgentCoreNative` is where most local product behavior now comes together. That
is good because it keeps CLI, ACP, and server surfaces aligned, but it also
means this type is the main pressure point whenever session controls,
capability discovery, or new runtime-facing semantics are added.
