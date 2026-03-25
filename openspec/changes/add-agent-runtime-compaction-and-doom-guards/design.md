# Design

## Core decision

Add compaction and doom-loop support as **runtime-backed primitives with
loop-controlled policy**.

That means:

- `agent-runtime` owns detection, state, safe transcript mutation, and optional
  hard-stop rails
- loops decide when to compact and how to react to advisory doom-loop warnings

## Compaction

Compaction remains a loop-triggered action through `LoopDecision::CompactContext`.
The runtime now supplies the missing mechanics behind that decision:

- transcript-pressure estimation from provider model limits
- summary subcall execution
- safe transcript rewriting
- compaction bookkeeping and events

The built-in `SimpleLoop` stays small and only uses runtime advice:

- if runtime says transcript pressure is high, request compaction
- otherwise continue normally

## Doom-loop handling

The runtime tracks repeated tool-call signatures after tool execution and
surfaces advisory state and `DoomLoopWarning` events.

Reaction is deliberately split:

- loops may ignore, steer, or escalate advisory warnings
- runtime may optionally enforce a configured hard-stop threshold

To support loop-owned recovery, the loop surface gains a local steering
decision.

## Documentation rule

Add a dedicated architecture doc for `agent-runtime` so future features keep
the same boundary:

- runtime owns mechanisms and invariants
- loops own orchestration policy
- prompts and higher-level strategy own semantic recovery wording and behavior
