# agent-cli

`agent-cli` is the main local application entrypoint. It bootstraps an embedded
`AgentCoreNative`, exposes interactive chat and session management through the
`agent` binary, and owns the direct benchmark execution path through
`agent exec`.

## Command Surface

| Command | Role |
| --- | --- |
| `agent` | Interactive local chat |
| `agent sessions ...` | Session discovery and resume |
| `agent credentials ...` | Credential management and OAuth login |
| `agent exec ...` | Non-interactive benchmark/harness execution with artifact output |
| `agent acp` | ACP stdio compatibility alias backed by `agent-acp` |

## Runtime Bootstrap

| Step | Behavior |
| --- | --- |
| 1 | Resolve config from defaults, global config, project config, and root `AGENTS.md` |
| 2 | Open a `FileStore` rooted at `agent_home()` |
| 3 | Discover providers from credentials, OAuth, local features, or mock fallback |
| 4 | Register loops: `simple`, `robust`, `terminus2`, `terminus-kira` |
| 5 | Build `AgentCoreNative` with the native tool executor via `native_tools()` |

## Two Modes

| Mode | What makes it different |
| --- | --- |
| Interactive mode | Uses stored credentials and persistent local state |
| `exec` mode | Accepts explicit auth and output settings for benchmark use; writes `run.json`, `events.jsonl`, and `trajectory.json` |

## CLI In Context

```mermaid
flowchart TD
    User[User or Harbor]
    CLI[agent-cli]
    Core[AgentCoreNative]
    Store[FileStore]
    Artifacts[run.json / events.jsonl / trajectory.json]

    User --> CLI
    CLI --> Core
    Core --> Store
    CLI --> Artifacts
```

## Architectural Reading

The CLI is no longer a thin demo wrapper. It is both the primary local user
surface and the direct benchmark surface that lets Harbor exercise the native
agent stack without ACP in the middle.
