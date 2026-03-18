# SWE-bench Family

## Overview

`SWE-bench` is still the central public benchmark family for software
engineering agents. It is the benchmark most external readers will recognize,
and it gives `brain` a coding-specific reference point that is broader than any
one product's internal evals.

| Item | Value |
| --- | --- |
| Primary domain | Real software engineering tasks from GitHub issues |
| Best use for `brain` | External coding baseline and regression target for loop changes |
| Arbitrary-agent fit | Medium |
| Harness-fit | Medium |
| Local-runnability | Good |
| Main limitation | Published results usually compare agent-plus-model bundles rather than isolating harness quality |

## What It Measures

At a high level, `SWE-bench` measures whether an agent can resolve real
repository issues in a way that passes evaluation checks. The family now
includes several useful variants:

- `Verified`: human-filtered subset intended to improve benchmark quality
- `Lite`: lower-cost subset for more practical evaluation
- `Multilingual`: broader language coverage
- `Multimodal`: issues with visual elements

## Why It Matters For AgentLoop

Even though `SWE-bench` is not a pure harness benchmark, it still stresses many
of the loop behaviors that matter for `brain`:

- codebase exploration
- iterative tool use
- patch generation
- error recovery
- stop conditions
- prompt and context discipline over multi-step tasks

This is also the benchmark most likely to make future `brain` loop work legible
to outside readers.

## Official Harness And Ecosystem

The benchmark itself is well-established, but the surrounding execution story is
still multi-layered:

- the official `SWE-bench` site centralizes variants and leaderboards
- `mini-SWE-agent` provides a deliberately small scaffold for reproducible runs
- `OpenHands` ships a broader evaluation harness that includes `SWE-bench`
- `Harbor` is actively adapting `SWE-bench` into its common evaluation layer

That ecosystem is good news for `brain`: it means we can choose whether we want
to treat `SWE-bench` as a benchmark to run directly or as a dataset consumed
through a more general harness.

## Same-Model / Different-Harness Fit

`SWE-bench` is only a medium-strength fit for harness comparison.

It is also only a medium-strength fit for arbitrary-agent evaluation unless we
choose the surrounding runner carefully. The benchmark is useful, but many
practical runs end up routed through benchmark-specific wrappers.

It is useful when:

- the model is held constant
- the exact subset is fixed
- the repo/image setup is identical
- the tool surface is reasonably comparable

It becomes much less fair when:

- one harness has a richer default tool stack
- one harness relies on benchmark-specific prompts or wrappers
- one system uses a completely different patching / retry / search policy
- different unofficial subsets or run budgets are compared

## Primary Confounders

- tool asymmetry between harnesses
- issue subset selection
- max-step and timeout differences
- prompt scaffolding differences
- repo-image or dependency setup drift
- benchmark wrappers that quietly add extra policy

## Practical Role Of `mini-SWE-agent`

`mini-SWE-agent` is especially relevant because it pushes in the direction
`brain` cares about: a smaller, more explicit scaffold around the same benchmark
rather than one giant product runtime. Its docs make the harness assumptions
concrete: bash, file edits, stepwise workflow, explicit completion signal, and a
minimal execution contract.

That makes it a good comparison point for `brain`'s eventual "simple but not
toy" loop.

## Can We Run It Locally?

Yes. Compared to browser and full computer-use benchmarks, `SWE-bench` is one
of the more practical choices for local work, especially if we avoid the full
benchmark initially.

The realistic first move is not the entire benchmark. It is:

- `SWE-bench Lite`, or
- a pinned subset of `SWE-bench Verified`

## Recommended Use For `brain`

- Use `SWE-bench` as a core external coding benchmark, not the only benchmark.
- Start with `Lite` or a pinned `Verified` subset before trying larger runs.
- Prefer execution paths that let us plug in Codex, OpenCode, Claude Code, and
  `brain` directly instead of only running through one benchmark-owned agent.
- Prefer a minimal and explicit wrapper when possible, to reduce product
  surface-area confounders.
- Report cost and runtime alongside resolve rate.

## Primary Sources

- SWE-bench leaderboard: <https://www.swebench.com/>
- mini-SWE-agent docs: <https://mini-swe-agent.com/latest/usage/mini/>
- OpenHands evaluation harness docs:
  <https://allhandsai.mintlify.app/openhands/usage/developers/evaluation-harness>
- OpenHands benchmarks repo: <https://github.com/OpenHands/benchmarks>
- Harbor adapter docs: <https://harborframework.com/docs/datasets/adapters>
- SWE-bench source: [`README.md`](../../repocache/SWE-bench/SWE-bench/README.md)
- SWE-bench harness code:
  [`swebench/harness`](../../repocache/SWE-bench/SWE-bench/swebench/harness)
- mini-SWE-agent source:
  [`src/minisweagent`](../../repocache/SWE-agent/mini-swe-agent/src/minisweagent)
- OpenHands SWE-bench benchmark:
  [`benchmarks/swebench`](../../repocache/OpenHands/benchmarks/benchmarks/swebench)
