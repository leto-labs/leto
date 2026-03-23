# Terminal-Bench

## Overview

`Terminal-Bench` is the strongest current benchmark fit for `brain`'s near-term
needs. It is explicitly about agents operating in terminal environments and it
ships a real harness designed to integrate multiple kinds of agents.

| Item | Value |
| --- | --- |
| Primary domain | Terminal-native agent tasks |
| Best use for `brain` | First serious apples-to-apples benchmark for Codex/OpenCode/`brain` style harnesses |
| Arbitrary-agent fit | Strong |
| Harness-fit | Strong |
| Local-runnability | Good |
| Main lesson | This is the clearest external benchmark that naturally matches a terminal-first coding-agent runtime |

## What It Measures

`Terminal-Bench` measures whether an agent can complete complex tasks in a
terminal environment. The benchmark covers broader terminal use than software
engineering alone, but that is a feature, not a bug, for `brain`:

- shell fluency
- multi-step workflows
- environment understanding
- tool chaining
- autonomous execution under constraints

The site currently describes both `Terminal-Bench 1.0` and `2.0`, with `2.0`
framed as harder and more heavily verified.

## Why It Matters For AgentLoop

This benchmark is unusually aligned with the loop concerns in
`harden-agent-loop`, and just as importantly it is one of the clearest public
benchmarks that does not force us into one benchmark-specific agent product:

- retries and recovery
- stopping conditions
- doom loops
- iterative tool use
- budget management
- environment-aware planning

It also matches the products we actually want to compare against: Codex,
OpenCode, OpenHands, Goose-style systems, and eventually `brain`.

## Official Harness

The launch post is unusually explicit that the harness is part of the value, not
just the dataset. The official infrastructure handles:

- agent orchestration
- multi-container Docker environments
- action logging
- environment verification

This is important because it lowers the amount of custom benchmark glue we need
to invent ourselves.

## Agent Integration Modes

The official docs describe three integration paths:

1. container installation
2. direct integrations through a Python interface
3. MCP server exposure

That flexibility is one of the main reasons `Terminal-Bench` is so relevant for
`brain`. It suggests we could later evaluate:

- a native `brain` integration
- an MCP-facing `brain` surface
- or a wrapper around an external agent process

without changing the benchmark itself.

## Harbor Relationship

The current `Terminal-Bench` story is increasingly tied to `Harbor`.
`Harbor` is positioned as a generalized framework for evaluating and optimizing
agents in container environments, and the adapter docs make an even stronger
point: the team wants benchmarks and agents to be portable into a common
evaluation harness with parity experiments.

That parity language matters a lot for `brain`. It is very close to the local
research question we care about: can we compare different harnesses fairly using
the same tasks, the same agent settings, and the same model?

## Same-Model / Different-Harness Fit

This is a strong benchmark for harness comparison because:

- the benchmark is terminal-first rather than tied to one product UX
- the harness is explicit and reusable
- multiple integration paths exist
- the surrounding ecosystem is moving toward parity experiments and adapters

It is still not perfectly clean. Confounders remain:

- some agents support only specific models
- different integration paths may expose different tool capabilities
- direct integrations may be stronger than install-in-container approaches

But compared to most public agent leaderboards, this is much closer to the
experimental setup `brain` wants:

- run any agent we care about
- keep the model fixed when possible
- otherwise compare the closest practical model pairings explicitly

## Can We Run It Locally?

Yes, with caveats. The main costs are operational:

- Docker/container setup
- benchmark-specific environments
- runtime and compute

Those are acceptable tradeoffs for a first benchmark because the benchmark is so
well aligned to our loop and tool model.

## Recommended Use For `brain`

- Make `Terminal-Bench` the first external benchmark we aim to support.
- Prefer one benchmark version and one pinned task slice at first, rather than
  chasing the whole leaderboard immediately.
- When later comparing harnesses, hold model, timeout, and step budget fixed
  when possible, and explicitly record any model mismatch when not.
- Watch `Harbor` closely before building any custom benchmark glue that would
  duplicate its adapter and parity concepts.

## Primary Sources

- Terminal-Bench home: <https://www.tbench.ai/>
- Terminal-Bench launch post: <https://www.tbench.ai/news/announcement>
- Terminal-Bench 2.0 and Harbor: <https://www.tbench.ai/news/announcement-2-0>
- Harbor framework: <https://harborframework.com/>
- Harbor adapter docs: <https://harborframework.com/docs/datasets/adapters>
- Terminal-Bench source: [`README.md`](../../../repocache/laude-institute/terminal-bench/README.md)
- Terminal-Bench runtime:
  [`terminal_bench`](../../../repocache/laude-institute/terminal-bench/terminal_bench)
- Terminal-Bench adapters:
  [`adapters`](../../../repocache/laude-institute/terminal-bench/adapters)
- Harbor source: [`README.md`](../../../repocache/harbor-framework/harbor/README.md)
- Harbor core: [`src/harbor`](../../../repocache/harbor-framework/harbor/src/harbor)
- Harbor adapters:
  [`adapters`](../../../repocache/harbor-framework/harbor/adapters)
