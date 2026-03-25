# Design: Bounded Declarative Loop Surface

## Core Rule

Keep:

- one concrete `agent-runtime`
- one `LoopStrategy` abstraction
- one declarative loop surface

Do not add:

- `If`
- `While`
- `ForEach`
- plan-level variables or registers
- nested workflow DSL objects

Advanced loops should work by:

1. reading richer runtime state
2. returning one concrete runtime-native effect
3. re-entering on the next tick with updated state

## New Effect Families

This change adds three bounded runtime-native effects:

- `RunSubcall`
  - pure provider call
  - tool-free by default
  - tagged with a stable purpose label
- `RewriteTranscript`
  - replace transcript with a loop-authored concrete transcript
- `AppendTranscriptMessages`
  - append runtime-authored concrete messages to the transcript
  - reject fabricated assistant messages

These effects are runtime-native operations, not control-flow constructs.

## Loop-Visible Operation State

Loops need a way to branch across ticks without a mini interpreter. The runtime
therefore records:

- `last_subcall`
- `last_transcript_rewrite`
- `last_transcript_append`
- bounded `recent_operations`

This keeps intermediate results in runtime state rather than in plan-level temp
variables.

## PTY Direction

Terminus-style loops need more than compaction and subcalls; they also need a
persistent PTY/session substrate. This change does not implement PTY-native
runtime effects yet, but it records the intended direction:

- near term: PTY may sit behind runtime-native effects layered over an
  underlying tool/driver
- longer term: PTY may evolve toward a runtime-managed resource with identity,
  lifecycle, structured observations, and safe interrupt/close behavior

PTY is intentionally future direction here, not a requirement for this change.

## Why Not A Programmable Loop Trait

Making the whole runtime a trait or introducing a second "programmable loop"
system would fragment the architecture too early. The preferred direction is:

- simple loops remain tiny
- advanced loops use the same declarative surface
- the surface grows by adding runtime-native effects and richer state, not by
  adding workflow control flow
