# AgentCoreRemote

`AgentCoreRemote` exists as the planned remote/shared-core client boundary. This
page documents the architectural slot it is expected to fill so the docs can
distinguish between the local core implementation and the remote one.

## Intended Role

| Concern | Expected direction |
| --- | --- |
| Core contract | Implement the same `AgentCore` trait used by local clients |
| Deployment model | Remote or attachable core boundary rather than in-process only |
| Client surfaces | Potential future `serve` / `attach`, richer remote TUI, or API-facing clients |
| Shared semantics | Reuse the same session, model, loop, tool, and event concepts as the native runtime |

## Design Guardrails

| Guardrail | Why it matters |
| --- | --- |
| Do not invent a second app model | CLI, ACP, and future remote clients should not each require their own runtime semantics |
| Preserve `AgentCore` as the contract | The trait is the point of convergence |
| Keep transport separate from runtime semantics | Remote delivery should not redefine projects, sessions, loops, or tools |

## Current Status

| Question | Answer |
| --- | --- |
| Implemented today? | Partially, as an active design/runtime direction |
| Documented to reserve the boundary? | Yes |
| Active local core implementation now? | [`BrainRuntimeNative.md`](BrainRuntimeNative.md) |
