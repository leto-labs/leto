# File Store

`FileStore` is the real durable store used by the local runtime.

## Characteristics

| Concern | Current behavior |
| --- | --- |
| Root location | uses `brain_home()` by default |
| Persistence | durable local filesystem state |
| Domains | projects, sessions, messages, credentials, and trajectories |
| Eventing | publishes lifecycle events for all major store domains |

## Why It Matters

`FileStore` is what makes session resume, credential reuse, project discovery,
and ATIF trajectory persistence work across CLI runs and ACP-backed local
sessions.
