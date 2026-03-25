# Proposal: Evolve Agent Loop Declarative Surface

## Why

The current `agent-runtime` / `agent-loops` split is already a strong fit for
simple policy loops, but advanced loops such as `terminus2` and
`terminus-kira` need more than `RunProvider`, `ExecuteToolBatch`,
`CompactContext`, and `QueueSteering`.

The goal is **not** to create a second programmable loop architecture and **not**
to turn `LoopDecision` into a workflow interpreter. The goal is to keep one
loop architecture while widening the declarative runtime-native effect surface
just enough to support richer multi-tick flows:

- isolated provider subcalls
- safe transcript rewrites
- safe transcript appends
- typed runtime operation results visible back to loops

This preserves the clarity of the declarative loop boundary while avoiding the
trap of pushing more and more hidden policy into `engine.rs`.

## What Changes

- add bounded runtime-native loop effects for:
  - `RunSubcall`
  - `RewriteTranscript`
  - `AppendTranscriptMessages`
- expose typed recent operation state in `SessionState`:
  - `last_subcall`
  - `last_transcript_rewrite`
  - `last_transcript_append`
  - bounded `recent_operations`
- keep simple loops as small-subset users of the same runtime surface
- document the design rule that advanced loops should branch in Rust across
  ticks, not through plan-level `If` / `While` / variables
- record PTY as the next runtime-native capability layer in architecture docs,
  but do not require PTY implementation in this change

## Impact

- `agent-runtime` becomes more expressive for advanced loops without becoming a
  general workflow engine
- `agent-loops` stays on one architecture
- Terminus-style future ports have a cleaner path: more runtime-native effects
  and more typed loop-visible state rather than more hidden runtime policy
