# Benchmark Methodology

This document defines how `brain` should read benchmark claims and how we
should later run our own comparisons.

## Core Principle

When the goal is to evaluate an agent harness, the most important property of a
benchmark is that it allows us to run the agents we actually care about through
a common evaluation path.

The ideal setup holds the harness as the main independent variable while also
holding the model constant. But the hard requirement is weaker:

- we must be able to run arbitrary agents, not just one benchmark-owned runtime
- we should get model parity as close as practical
- when exact parity is impossible, we should make the mismatch explicit rather
  than rejecting the benchmark entirely

That sounds obvious, but most public agent benchmark results still conflate:

- model choice
- provider endpoint and settings
- prompt scaffold
- tool policy
- runtime budget
- benchmark harness
- environment reset behavior

## Definitions

### Model

The underlying model identifier and provider endpoint, for example
`gpt-5.4` via a specific OpenAI-compatible API.

### Harness

The full agent runtime around the model:

- prompt construction
- loop logic
- tool execution
- retries and error handling
- planning modes
- context compaction
- approvals and sandbox policy
- stopping conditions

For `brain`, this mostly maps to `AgentLoop` plus the prompt/tool/runtime policy
around it.

### Benchmark

The task set, environment, and scoring harness used to evaluate the agent.

## Standard Comparison Rules

Any future `brain` benchmark run should try to hold these constant where
possible:

1. Same model ID and same provider endpoint
2. Same temperature and other inference settings where the benchmark allows it
3. Same benchmark split or fixed task subset
4. Same attempt policy per task
5. Same timeout and max-step budget
6. Same sandbox/tool permissions where the agent surface allows normalization
7. Same scoring harness and environment-reset logic
8. Repeated runs for stochastic systems when cost is acceptable

If exact model parity is not possible, the run is still useful if:

- the benchmark supports all target agents cleanly
- the chosen models are the closest realistic peers available for each harness
- the mismatch is reported explicitly

Example comparison matrix that is still valid for `brain`:

- Codex with `codex-5.4`
- Claude Code with `Opus`
- OpenCode with both where supported
- `brain` with both where supported

This is weaker than a strict same-model study, but still much more useful than
a benchmark that only permits one custom harness.

## Standard Metadata For Benchmark Pages

Each benchmark page in this folder should answer the same questions:

| Field | Meaning |
| --- | --- |
| `What it measures` | Core capability under test |
| `Why it matters for AgentLoop` | Which loop behaviors the benchmark stresses |
| `Official harness` | Whether there is a canonical evaluation path |
| `Can we run arbitrary agents?` | Whether the benchmark lets us plug in the agents we care about |
| `Can we run locally?` | Practical local run outlook |
| `Can we compare same-model / different-harness?` | How fair apples-to-apples comparisons can be |
| `Primary confounders` | What most undermines fairness |
| `Recommended use for brain` | Concrete role in our future eval stack |

## Rating Scale

### Harness-fit

- `Strong`: we can run arbitrary agents through a common harness with good
  control over budgets and policies
- `Medium`: we can likely run multiple agents, but some wrappers or benchmark
  assumptions still distort fairness
- `Weak`: the benchmark mostly presumes one runtime or makes cross-agent
  integration awkward enough that the comparison quality drops sharply

### Local-runnability

- `Good`: open and feasible on a serious dev machine
- `Medium`: open, but operationally heavy or annoying
- `Heavy`: technically open, but expensive or cumbersome enough that it should
  not be our first local target
- `Poor`: not realistically self-servable

## What To Prefer

- Prefer benchmarks that let us plug in arbitrary agents over benchmarks that
  quietly require a benchmark-specific runtime.
- Prefer benchmark setups that support ablations of loop/tool design, not just
  one final bundled score.
- Prefer official scoring harnesses over ad hoc scripts.
- Prefer fixed benchmark subsets over unpinned "sample some tasks."
- Prefer benchmarks with environment reset and verification built in.
- Prefer benchmarks that let us plug multiple agent types into one evaluator.
- Prefer cost-aware reporting, not just success rate.

## What To Record In Future Runs

For each harness/model/benchmark tuple, record at minimum:

- benchmark name and version
- task subset or instance IDs
- model ID
- provider endpoint
- harness name and version / git SHA
- tool/sandbox policy summary
- max steps
- timeout per task
- success rate
- total cost
- wall-clock time
- mean steps / tool calls per task
- failure notes

## Benchmark-Specific Confounders To Watch

### Coding benchmarks

- repository/image setup differences
- patch-application conventions
- tool set asymmetry across harnesses
- issue-selection drift or unofficial subsets

### Terminal benchmarks

- shell access differences
- background-process handling
- tmux or PTY assumptions
- container-install versus direct integration paths

### Browser / computer-use benchmarks

- environment freshness
- site drift
- OCR / perception stack differences
- browser wrapper quality dominating the loop itself

## Local Evaluation Defaults For `brain`

If we later stand up local comparisons, default to:

- fixed model and provider for all harnesses under test when feasible
- closest practical model parity when exact parity is not feasible
- one benchmark at a time
- one pinned subset at a time
- success rate plus cost and runtime, never success rate alone
- at least three runs when the benchmark is noisy enough to justify it
- no homemade scoring if the benchmark already ships one

## Working Recommendation

The first goal should not be "build our own grand unified leaderboard." The
first goal should be a small, defensible, reproducible comparison loop:

- `Terminal-Bench`
- then `SWE-bench Lite` or a pinned `Verified` subset
- with the same model when possible, and explicit nearest-peer models otherwise
- under explicit budget and policy controls
- plus a small set of internal ablations:
  bash-only versus typed tools, stateless shell versus PTY, and linear-history
  export versus compaction-aware runtime state

That is enough to start learning whether `brain`'s next `AgentLoop` actually
improves outcomes.
