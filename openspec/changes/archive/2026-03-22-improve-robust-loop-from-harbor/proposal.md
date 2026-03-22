# Improve Harbor Failure Surfacing From Robust Loop Findings

## Why

The first full `brain-acp` robust run on `terminal-bench@2.0` established that
the current main bottleneck is no longer Harbor portability. The musl artifact
fix worked across the mixed task-image set.

The remaining failures are now concentrated in loop and ACP behavior:

- iteration-budget exhaustion currently ends up looking like Harbor
  `RequestError`
- long-running tool steps and long tasks are hard to reason about while they
  are in flight
- the loop often finishes in the wrong way even when the task diagnosis is
  partly correct

The benchmark findings should be preserved in docs, and the most obviously
incorrect ACP failure surfacing should be fixed so loop exhaustion stops looking
like a transport failure.

## What Changes

- expand the benchmark loop analysis in `docs/benchmarks/LoopDesign.md`
- update the ACP backend so normal loop termination outcomes do not surface as
  ACP internal protocol failures
- capture that ACP behavior explicitly in OpenSpec

## Impact

- benchmark findings become durable internal design input rather than chat-only
  history
- Harbor failures caused by loop-budget exhaustion stop masquerading as generic
  request failures
- follow-on loop-hardening work can build on a clearer benchmark baseline
