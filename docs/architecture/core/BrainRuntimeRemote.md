# BrainRuntimeRemote

`BrainRuntimeRemote` does not exist yet. This page documents the architectural
slot it is expected to fill later so the current docs can distinguish between
the runtime contract and the only implementation that exists today.

## Intended Role

| Concern | Expected direction |
| --- | --- |
| Runtime contract | Implement the same `BrainRuntime` trait used by local clients |
| Deployment model | Remote or attachable runtime boundary rather than in-process only |
| Client surfaces | Potential future `serve` / `attach`, richer remote TUI, or API-facing clients |
| Shared semantics | Reuse the same session, model, loop, tool, and event concepts as the native runtime |

## Design Guardrails

| Guardrail | Why it matters |
| --- | --- |
| Do not invent a second app model | CLI, ACP, and future remote clients should not each require their own runtime semantics |
| Preserve `BrainRuntime` as the contract | The trait is the point of convergence |
| Keep transport separate from runtime semantics | Remote delivery should not redefine projects, sessions, loops, or tools |

## Current Status

| Question | Answer |
| --- | --- |
| Implemented today? | No |
| Documented to reserve the boundary? | Yes |
| Active runtime implementation now? | [`BrainRuntimeNative.md`](BrainRuntimeNative.md) |
