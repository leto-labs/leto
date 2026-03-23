# brain-transports

`brain-transports` is intentionally small. It is the crate for local
human-facing transport implementations that speak the `Transport` trait from
`brain-types`.

## Current Surface

| Type | Role |
| --- | --- |
| `CliTransport` | stdin/stdout transport for interactive local use |

## Relationship To Other Surfaces

| Surface | Where it belongs |
| --- | --- |
| Interactive local CLI | `brain-transports` |
| Native runtime bus | `brain-types` / `brain-core` |
| ACP client/server mapping | `brain-acp` |
| Deferred remote server APIs | `brain-server` |

## Architectural Reading

The transport crate stays thin because the repo has shifted toward
`BrainRuntime` as the app-facing boundary. Richer UX concerns now tend to sit
above the runtime rather than inside the transport crate itself.
