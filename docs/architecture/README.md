# Architecture Overview

This section documents the current `brain` workspace using the live
`agent-*` / `provider-*` naming.

## Stack At A Glance

| Layer | Crates | Role in the stack | Current status |
| --- | --- | --- | --- |
| Shared contracts | [`types.md`](types.md) | Where current shared contracts are split across `provider`, `agent-runtime`, and `agent-store` | Active foundation |
| Application core | [`core/README.md`](core/README.md) | `AgentCore`, `AgentCoreNative`, and the future remote core shape | Core orchestration |
| Reusable session engine | [`agent-runtime/README.md`](agent-runtime/README.md) | Live per-session engine and loop/runtime boundary | Active foundation |
| Execution strategies | [`loops/README.md`](loops/README.md) | `simple`, `robust`, `terminus2`, `terminus-kira` | Highest churn / experimental pressure |
| Runtime capabilities | [`tools/README.md`](tools/README.md), [`provider/README.md`](provider/README.md), [`stores/README.md`](stores/README.md), [`transports.md`](transports.md), [`config.md`](config.md) | Tool execution, provider integrations, persistence, thin transports, and bootstrap config | Mixed maturity |
| Client surfaces | [`cli.md`](cli.md), [`acp.md`](acp.md), [`server/README.md`](server/README.md) | Local CLI, ACP adapter, and hosted server surfaces | Active product surfaces |
| Benchmark/export surfaces | [`atif.md`](atif.md) | ATIF transcript/export model for Harbor-facing runs | Important integration layer |

## System Map

```mermaid
flowchart TD
    CLI[agent-cli]
    ACP[agent-acp]
    Harbor[Harbor direct + ACP runs]
    Server[agent-server]
    Core[agent-core::AgentCoreNative]
    Session[agent-runtime::SessionEngine]

    Provider[provider / provider-*]
    Loops[agent-loops]
    Tools[agent-tools]
    Store[agent-store]
    Atif[atif]

    CLI --> Core
    CLI --> Atif
    ACP --> Core
    Server --> Core
    Harbor --> CLI
    Harbor --> ACP
    Core --> Session
    Core --> Provider
    Core --> Loops
    Core --> Tools
    Core --> Store
    Core --> Atif
    Session --> Provider
    Session --> Tools
```

## Runtime Turn Flow

```mermaid
sequenceDiagram
    participant Client as CLI / ACP client
    participant Core as AgentCoreNative
    participant Store as Store
    participant Loop as LoopStrategy
    participant Provider as Provider
    participant Tool as ToolExecutor
    participant Atif as ATIF builder

    Client->>Core: turn(session_id, input)
    Core->>Store: load session + project config
    Core->>Core: resolve provider, loop, tools
    Core->>Loop: drive runtime turn
    Loop->>Provider: chat(messages, tools, inference)
    Provider-->>Loop: token deltas / tool calls / usage
    Loop->>Tool: execute(...) as needed
    Tool-->>Loop: tool result
    Loop-->>Core: Event stream
    Core->>Store: persist user/assistant/tool messages
    Core->>Atif: append trajectory state and completion events
    Core-->>Client: live events + TurnDone
```

## Design Reading

| Question | Primary doc |
| --- | --- |
| What are the canonical shared contracts? | [`types.md`](types.md) |
| What is the difference between `AgentCore` and `agent-runtime`? | [`core/README.md`](core/README.md) |
| What belongs in `agent-runtime` versus a loop strategy? | [`agent-runtime/README.md`](agent-runtime/README.md) |
| Where are the experimental loop ideas concentrated? | [`loops/README.md`](loops/README.md) |
| How do tools stay swappable across native and ACP-backed execution? | [`tools/README.md`](tools/README.md) |
| How do provider backends handle presets, auth, and multimodal requests? | [`provider/README.md`](provider/README.md) |
| How is durable state stored and observed? | [`stores/README.md`](stores/README.md) |
| How does config get layered from defaults, global state, and project files? | [`config.md`](config.md) |
| How do CLI, ACP, and the server sit on top of the core? | [`cli.md`](cli.md), [`acp.md`](acp.md), [`server/README.md`](server/README.md) |
| How does Harbor-facing transcript export work? | [`atif.md`](atif.md) |
| How should OpenCode compatibility docs be shaped? | [`server/opencode.md`](server/opencode.md) |

## Current Architectural Reading

| Area | Healthy pattern | Current pressure point |
| --- | --- | --- |
| Core boundary | `agent-core` keeps product-facing orchestration separate from the live session engine | More behavior is now traveling through events, approvals, PTYs, and child agents |
| Runtime assembly | `AgentCoreNative` gives one place to resolve providers, loops, and tools | More responsibilities are accumulating around bootstrap, credentials, and client integration |
| Loop experimentation | Loop work is isolated in `agent-loops` instead of hardcoding policy in `agent-core` | `terminus2` and `terminus-kira` are materially different from `simple`/`robust` |
| Tooling | Driver pattern keeps tool contracts stable while swapping execution backends | ACP-backed file access and persistent terminal sessions expand the contract surface |
| Benchmarks | `agent exec` and `atif` give a direct benchmark/export path | Harbor and loop work are influencing architecture faster than the docs had kept up |
