# brain-server

`brain-server` is legacy server-side infrastructure that remains in the repo
but is not the active application boundary for current local surfaces.

## What It Contains

| Component | Role |
| --- | --- |
| `BrainApi` | Older app-facing API abstraction |
| `BrainServer` | Server wrapper around `Brain` |
| `EventBus` | Broadcast bus for `ServerEvent` |
| HTTP helpers | Router/build/serve utilities |
| Server types | Request, status, and event wrappers |

## Current Status

| Question | Answer |
| --- | --- |
| Is it the main runtime surface today? | No |
| Do CLI and ACP depend on it? | No; they depend on `BrainRuntime` / `BrainRuntimeNative` |
| Why keep it? | It preserves deferred server architecture work and older remote-runtime experiments |

## Architectural Reading

The important thing is not that `brain-server` exists. The important thing is
that the active repo direction has moved away from it as the primary client
boundary. Treat it as deferred legacy infrastructure until a new remote-runtime
design is chosen.
