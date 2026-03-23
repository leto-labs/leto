# Research Gaps And Future Local Program

## Main Research Finding

The public benchmark ecosystem is improving, but it still does not cleanly
answer the question `brain` cares about most:

> If we keep the model fixed, how much better is harness A than harness B?

Most public results still compare:

- harness plus model
- harness plus benchmark-specific wrappers
- harness plus proprietary product features

That makes public leaderboards informative, but not decisive.

## The Biggest Gaps

### 1. Agent-plus-model bundling

Most public scores are reported as one combined system, not as one model run
through multiple equivalent harnesses.

### 2. Tool-surface asymmetry

Two agents can both "run on the same benchmark" while exposing meaningfully
different tool affordances, approval policies, or environment visibility.

### 3. Budget mismatch

Max steps, retries, timeout, and cost ceilings often differ across systems.

### 4. Benchmark-specific wrappers

A benchmark may look general-purpose while in practice being easiest to run
through one benchmark-specialized runtime.

### 5. Reproducibility versus leaderboard visibility

Some leaderboards are easy to read but hard to reproduce exactly, especially for
larger agent systems.

## What `brain` Should Do Instead

Treat external benchmarks as inputs to an internal evaluation methodology rather
than as turnkey truth.

The future local program should be:

1. fixed model
2. fixed provider endpoint
3. fixed benchmark slice
4. fixed budget
5. explicit tool/sandbox policy
6. repeated runs where stochasticity matters
7. success rate plus cost and runtime, never success rate alone

When exact model parity is not feasible across all harnesses, the fallback is:

1. keep the benchmark and task slice fixed
2. use the closest practical model pairing for each harness
3. report the pairing explicitly
4. avoid pretending it was a pure same-model comparison

## Proposed Local Evaluation Sequence

### Phase 1: Terminal-first harness comparison

Benchmark:
`Terminal-Bench`

Goal:
compare terminal-native harness quality across systems that can all run through
the same benchmark, using the same model when possible or the nearest practical
model pairing otherwise.

Deliverables:

- success rate
- runtime
- cost
- failure taxonomy

### Phase 2: Coding-benchmark grounding

Benchmark:
`SWE-bench Lite` or a pinned `Verified` subset

Goal:
check whether loop improvements that help on terminal tasks also help on
recognized software engineering tasks.

Deliverables:

- resolve rate
- cost
- tool-call counts
- common failure modes

### Phase 3: Adjacent stress tests

Benchmarks:

- `OSWorld`
- `WebArena` / `VisualWebArena`
- `tau-bench`
- `AppWorld`

Goal:
measure whether the same loop design generalizes beyond terminal-centric coding.

## What We Should Avoid Early

- building a custom all-in-one benchmark runner before we have benchmark
  priorities
- mixing many benchmarks into one score too early
- chasing full public leaderboard parity before we can reproduce small local
  slices
- comparing systems with different models and calling it a pure same-model
  harness study
- benchmarks that only work well through one custom benchmark-owned agent

## Working Recommendation

For `brain`, the real benchmark milestone is not "get on every leaderboard." It
is:

- run one defensible terminal benchmark
- run one defensible coding benchmark
- keep model and budget fixed
- use the results to guide the next `AgentLoop` design

That is enough to turn benchmark research into engineering signal.
