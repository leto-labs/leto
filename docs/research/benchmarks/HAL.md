# HAL

## Overview

`HAL` is not a benchmark dataset in the same sense as `SWE-bench` or
`Terminal-Bench`. It is a standardized, cost-aware evaluation layer for agents
across multiple benchmarks.

That makes it one of the most strategically important pieces of infrastructure
for `brain`, even if it is not the first thing we will run locally.

| Item | Value |
| --- | --- |
| Primary role | Multi-benchmark evaluation infrastructure and leaderboard |
| Best use for `brain` | Normalization layer for future harness comparisons |
| Arbitrary-agent fit | Strong |
| Harness-fit | Strong in principle |
| Local-runnability | Medium |
| Main lesson | The strongest public work aimed at framework-agnostic, reproducible, cost-aware agent comparison |

## What It Measures

`HAL` aggregates multiple agent benchmarks under one evaluation umbrella. The
site positions itself as:

- standardized
- cost-aware
- third-party
- reproducible

It already spans several benchmark families, including `SWE-bench Verified
Mini`, `TAU-bench`, `CORE-Bench`, `ScienceAgentBench`, `GAIA`, and others.

## Why It Matters For AgentLoop

The interesting thing about `HAL` is not any one leaderboard score. It is the
structure:

- one harness for multiple benchmarks
- explicit cost-awareness
- framework-agnostic evaluation
- emphasis on reproducibility and unbiased comparison

Those are exactly the missing ingredients in most public "agent benchmark"
stories.

## Why It Is More Than Just Another Leaderboard

The main `HAL` site explicitly says agent developers should be able to:

- reproduce existing agents
- perform unbiased comparisons
- use the `HAL` harness for framework-agnostic agent evaluation

That language is unusually close to the experimental goal behind
`docs/research/benchmarks`: not just "who is best?" but "how do we compare
systems fairly?"

## Same-Model / Different-Harness Fit

`HAL` is one of the strongest public fits for that question, at least in
principle.

Why only "in principle"?

- benchmark-specific integrations still matter
- agent wrappers can still leak benchmark-specific assumptions
- some leaderboards still reflect product-plus-model bundles

Even so, `HAL` is one of the few public efforts that is visibly trying to build
the right meta-layer for harness comparison, and it is especially valuable to
`brain` because it is trying to host multiple agent implementations rather than
locking the benchmark to one harness.

## Can We Run It Locally?

Probably, but it should not be our first operational target. `HAL` is better
thought of initially as:

- a methodology source
- a normalization source
- a "what good evaluation infrastructure looks like" source

It becomes more compelling for local execution once `brain` already has one or
two benchmark integrations worth standardizing.

## Recommended Use For `brain`

- Treat `HAL` as the best public reference for how a future `brain`
  benchmarking program should look.
- Borrow its framing: reproducible, cost-aware, third-party-style evaluation.
- Do not wait for full `HAL` integration before running any benchmarks.
- Revisit `HAL` once `brain` has at least one stable benchmark adapter worth
  normalizing.

## Primary Sources

- HAL home: <https://hal.cs.princeton.edu/>
- HAL paper link from the site: <https://arxiv.org/abs/2510.11977>
- HAL source: [`README.md`](../../../repocache/princeton-pli/hal-harness/README.md)
- HAL core:
  [`hal`](../../../repocache/princeton-pli/hal-harness/hal)
- HAL benchmark definitions:
  [`hal/benchmarks`](../../../repocache/princeton-pli/hal-harness/hal/benchmarks)
- HAL example agents:
  [`agents`](../../../repocache/princeton-pli/hal-harness/agents)
