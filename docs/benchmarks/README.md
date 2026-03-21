# Benchmark Research

Source-first internal notes for benchmarks that matter to `brain`'s next
agent-loop iteration.

This folder is intended to answer one concrete engineering question for
`brain`:

> Which public benchmarks are actually useful for comparing agent harnesses,
> not just raw models, and which of them could we realistically run later
> against Codex, OpenCode, and eventually `brain` itself?

Short answer, as of 2026-03-18: there is no single widely adopted benchmark
whose primary design goal is "same model, different harness" comparison. But
there are several strong building blocks.

## Why This Matters For `brain`

`brain` already has the right top-level seam for this problem: `AgentLoop` is a
trait, not a hardcoded runtime. That means benchmark work should help us answer
questions like:

- does a stronger loop materially improve results when the model stays fixed?
- which loop features matter most: retries, compaction, doom-loop detection,
  tool policy, prompting, or planning modes?
- which public benchmarks are fair enough to compare Codex/OpenCode/`brain`
  style harnesses without mostly measuring unrelated product surface area?

If we do this well, benchmark research becomes a design input for
`harden-agent-loop`, not a disconnected doc dump.

## Focus Areas

This research concentrates on the dimensions that matter most for `brain`:

- coding-agent relevance
- terminal-first versus browser-first environments
- arbitrary-agent pluggability
- same-model / different-harness comparability when feasible
- openness and local run feasibility
- official harness quality
- cost and iteration budget controls
- how much a benchmark measures loop quality versus product integration

## Index

- [`Methodology.md`](Methodology.md)
- [`Formats.md`](Formats.md)
- [`LoopDesign.md`](LoopDesign.md)
- [`Harbor.md`](Harbor.md)
- [`SWEBench.md`](SWEBench.md)
- [`TerminalBench.md`](TerminalBench.md)
- [`HAL.md`](HAL.md)
- [`Adjacent.md`](Adjacent.md)
- [`Gaps.md`](Gaps.md)

## Executive Summary

- `Terminal-Bench` is currently the strongest direct fit for `brain` because it
  is explicitly about agents operating in terminal environments and ships a
  real harness for integrating multiple agent styles.
- `SWE-bench` remains unavoidable for coding-agent evaluation, especially
  `Lite` and `Verified`, but most published results still compare
  model-plus-harness bundles rather than isolating harness quality.
- The most important benchmark property for `brain` is not strict same-model
  purity. It is whether we can run arbitrary agents through a common benchmark
  harness instead of being forced into one benchmark-owned agent runtime.
- `mini-SWE-agent` is especially important because it shows there is growing
  interest in minimal, reproducible agent scaffolds that hold the surrounding
  harness constant while swapping models.
- mini-SWE-agent should be read as evidence for a strong benchmark baseline,
  not as proof that bash-only and linear-history loops are the best long-term
  runtime architecture.
- `HAL` is not a benchmark dataset by itself. It is more interesting for
  `brain` as standardized evaluation infrastructure: framework-agnostic,
  cost-aware, multi-benchmark, and explicitly aimed at reproducible agent
  comparisons.
- Harbor should be treated as the practical operational harness for the first
  benchmark workflow: it now uses five reference surfaces in this repo
  (`codex`, `mini-swe-agent`, `terminus-2`, `codex-acp`, `brain-acp`) through
  the dedicated runner script `scripts/harbor-run.sh` and the thin
  `just harbor-run <agent> <dataset> [task-name]` wrapper. The current common
  checkpoint ladder is `hello-world@1.0`, `regex-log`, `chess-best-move`, and
  `sqlite-with-gcov`, but the runner itself accepts arbitrary Harbor
  dataset/task combinations. Built-in `codex` is the stable baseline,
  `codex-acp` is the main repo-local ACP comparison path, `mini-swe-agent` and
  `terminus-2` are useful built-in comparison surfaces, and `brain-acp` is now
  real but still treated as a bounded validation path rather than the first
  stable real comparison baseline.
- Harbor's `ATIF` is the strongest current candidate for a reusable trace
  interchange format because it is designed for debugging, visualization, SFT,
  and RL rather than only one benchmark leaderboard.
- Other format families matter too: typed response-item streams, tool metadata,
  and structured patch artifacts are all useful inputs to loop design even when
  they are not "benchmark formats" in the narrow sense.
- Existing result and trace formats suggest `brain` should separate:
  benchmark-friendly aggregate results, rich resumable runtime logs, and
  exportable training trajectories.
- `OSWorld`, `WebArena` / `VisualWebArena`, `tau-bench`, and `AppWorld` are
  relevant adjacent benchmarks because they stress long-horizon orchestration,
  tool use, and environment interaction, but they are weaker first picks for a
  terminal-native coding agent.

## Ecosystem Snapshot

All rows below reflect the state observed on 2026-03-18.

| Benchmark / framework | Primary domain | Best use for `brain` | Arbitrary-agent fit | Same-model / different-harness fit | Local run outlook |
| --- | --- | --- | --- | --- |
| `Terminal-Bench` | Terminal agents | First serious benchmark for Codex/OpenCode/`brain` style systems | Strong | Strong | Good, but Docker/container-heavy |
| `SWE-bench` family | Software engineering | Core coding benchmark and external reference point | Medium | Medium | Good, especially on `Lite` or fixed subsets |
| `mini-SWE-agent` | Minimal SWE scaffold | Useful normalization layer for model-vs-harness analysis | Medium | Strong | Good |
| `HAL` | Evaluation infrastructure | Best common harness to watch for future apples-to-apples studies | Strong | Strong in principle | Medium; more infrastructure than quick local eval |
| `OSWorld` | Computer use | Stress test for long-horizon environment interaction | Medium | Medium | Medium to heavy |
| `WebArena` / `VisualWebArena` | Web agents | Good adjacent signal for planning and tool orchestration | Medium | Medium | Heavy |
| `tau-bench` | Tool-agent-user interaction | Good for tool policy and dialog loops | Medium | Medium | Medium |
| `AppWorld` | Interactive coding across apps | Interesting long-horizon adjacent benchmark | Medium | Medium | Heavy |

## Recommended Pilot Sequence

### 1. Start with terminal-native apples-to-apples

- primary target: `Terminal-Bench`
- objective: compare mature harnesses in a benchmark that can host arbitrary
  agents, using the same model when feasible and the closest practical model
  parity when it is not

### 2. Add software-engineering grounding

- primary target: `SWE-bench Lite` or a pinned subset of `SWE-bench Verified`
- objective: measure whether loop improvements transfer to a benchmark the
  broader coding-agent ecosystem already recognizes

### 3. Watch normalization infrastructure

- primary target: `HAL` and `Harbor`
- objective: reuse a standardized harness where possible instead of building a
  one-off internal leaderboard too early

### 4. Add adjacent stress tests only after the first two work

- `OSWorld`
- `WebArena` / `VisualWebArena`
- `tau-bench`
- `AppWorld`

## Working Conclusion

There is enough benchmark infrastructure today to begin serious harness
evaluation work for `brain`, but not enough to outsource the methodology
entirely. Public leaderboards still mostly compare agent-plus-model bundles.

The practical path for `brain` is:

- treat `Terminal-Bench` as the best first external benchmark
- treat `SWE-bench` as the unavoidable coding baseline
- treat `HAL` and `Harbor` as the most promising common evaluation layer
- prefer benchmarks that let us plug in Codex, Claude Code, OpenCode, and
  `brain` directly over benchmarks that effectively force one custom runtime
- treat adjacent benchmarks as loop stress tests, not the first benchmark story

## Primary Sources

- Terminal-Bench home: <https://www.tbench.ai/>
- Terminal-Bench launch post: <https://www.tbench.ai/news/announcement>
- Terminal-Bench 2.0 and Harbor: <https://www.tbench.ai/news/announcement-2-0>
- Harbor framework: <https://harborframework.com/>
- Harbor adapter docs: <https://harborframework.com/docs/datasets/adapters>
- SWE-bench leaderboard: <https://www.swebench.com/>
- mini-SWE-agent docs: <https://mini-swe-agent.com/latest/usage/mini/>
- HAL home: <https://hal.cs.princeton.edu/>
- OSWorld: <https://os-world.github.io/>
- VisualWebArena repo: <https://github.com/web-arena-x/visualwebarena>
- OpenHands evaluation harness docs:
  <https://allhandsai.mintlify.app/openhands/usage/developers/evaluation-harness>
- OpenHands benchmarks repo: <https://github.com/OpenHands/benchmarks>
- AppWorld: <https://appworld.dev/>
- Terminal-Bench source: [`repocache/laude-institute/terminal-bench`](../../repocache/laude-institute/terminal-bench)
- Harbor source: [`repocache/harbor-framework/harbor`](../../repocache/harbor-framework/harbor)
- SWE-bench source: [`repocache/SWE-bench/SWE-bench`](../../repocache/SWE-bench/SWE-bench)
- mini-SWE-agent source: [`repocache/SWE-agent/mini-swe-agent`](../../repocache/SWE-agent/mini-swe-agent)
- HAL source: [`repocache/princeton-pli/hal-harness`](../../repocache/princeton-pli/hal-harness)
- OSWorld source: [`repocache/xlang-ai/OSWorld`](../../repocache/xlang-ai/OSWorld)
- VisualWebArena source: [`repocache/web-arena-x/visualwebarena`](../../repocache/web-arena-x/visualwebarena)
- AppWorld source: [`repocache/StonyBrookNLP/appworld`](../../repocache/StonyBrookNLP/appworld)
- OpenHands benchmarks source: [`repocache/OpenHands/benchmarks`](../../repocache/OpenHands/benchmarks)
- OpenHands SDK source:
  [`repocache/OpenHands/software-agent-sdk`](../../repocache/OpenHands/software-agent-sdk)
