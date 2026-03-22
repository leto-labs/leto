# Design

## Benchmark Findings We Are Carrying Forward

The current Harbor full-suite result for `brain-acp` robust is:

- `24/89` passes
- `48/89` verifier failures
- `23/89` trials with exceptions

The three most actionable classes are:

1. iteration-budget exhaustion currently surfacing as Harbor `RequestError`
2. agent timeouts on long-running tasks or shell steps
3. clean verifier failures where the turn completed normally but the result was
   wrong

This change does not try to turn all of those findings into implemented loop
policy. The loop-side findings are documented analysis for the next design
pass, not normative behavior in this archive.

## ACP-Side Changes To Spec

The current ACP backend maps non-internal loop failures too aggressively to ACP
internal errors. In particular, `MaxIterations` currently falls through to
`internal_error()`, which is why Harbor reports those trials as `RequestError`.

This change should therefore implement and specify that:

- `MaxIterations`
- `Cancelled`

stop being surfaced as ACP internal errors.

Instead, ACP should:

- complete the prompt request normally
- preserve the emitted loop error events
- expose the termination reason as a normal turn-level outcome rather than a
  protocol failure

## Explicitly Deferred In This Change

The Harbor run still suggests a need for stronger loop changes:

- fewer “diagnose correctly, stop too early” turns
- better final validation before ending the turn
- more bounded final shell verification
- better stall detection and long-running tool visibility

Those remain documented analysis for now, not normative requirements in this
change.
