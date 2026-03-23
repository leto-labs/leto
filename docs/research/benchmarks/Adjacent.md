# Adjacent Benchmarks

These benchmarks are not the first benchmarking surface `brain` should pursue,
but they matter because they stress loop behaviors that a more advanced
`AgentLoop` will eventually need to handle.

## OSWorld

| Item | Value |
| --- | --- |
| Domain | Open-ended computer-use tasks |
| Harness-fit | Medium |
| Local-runnability | Medium to heavy |
| Why it matters | Strong stress test for long-horizon environment interaction |

`OSWorld` is useful when the question becomes broader than coding:
can the loop sustain multi-step environment interaction under realistic
conditions? The project now points readers to an explicit agent interface and
environment interface, which is a good sign for custom integrations. It also
offers a verified public-evaluation path, but that process is more involved than
what we want for a first local benchmark.

Recommendation for `brain`:
use later as a stress test for environment interaction and recovery, not as the
first benchmark story.

Primary source:
<https://os-world.github.io/>

Local source:
[`README.md`](../../../repocache/xlang-ai/OSWorld/README.md),
[`desktop_env`](../../../repocache/xlang-ai/OSWorld/desktop_env),
[`mm_agents`](../../../repocache/xlang-ai/OSWorld/mm_agents)

## WebArena / VisualWebArena

| Item | Value |
| --- | --- |
| Domain | Web-agent and multimodal web-agent tasks |
| Harness-fit | Medium |
| Local-runnability | Heavy |
| Why it matters | Good benchmark for planning, navigation, and multi-step action policies |

`VisualWebArena` is a realistic, execution-based benchmark built on top of the
`WebArena` line of work. The repo is open and includes environment setup,
evaluation scripts, and tests, which is good. The downside is operational:
website setup, browser automation, environment variables, and potentially GPU
requirements for some multimodal setups.

This is valuable for `brain` once browser or computer-use capabilities become
core. It is not the first benchmark to use when the primary design question is
"is our terminal/coding harness improving?"

Primary source:
<https://github.com/web-arena-x/visualwebarena>

Local source:
[`README.md`](../../../repocache/web-arena-x/visualwebarena/README.md),
[`evaluation_harness`](../../../repocache/web-arena-x/visualwebarena/evaluation_harness),
[`browser_env`](../../../repocache/web-arena-x/visualwebarena/browser_env)

## tau-bench

| Item | Value |
| --- | --- |
| Domain | Tool-agent-user interaction |
| Harness-fit | Medium |
| Local-runnability | Medium |
| Why it matters | Strong adjacent benchmark for tool policy, dialog loops, and simulated user interaction |

`tau-bench` is not a coding benchmark, but it is very relevant to loop design:
it measures an agent interacting with users and tools in realistic domains.
That makes it interesting for:

- turn structure
- tool invocation policy
- user clarification loops
- controlled interaction under policy constraints

Recommendation for `brain`:
consider after the first coding/terminal benchmarks, especially if `brain`
starts supporting richer human-in-the-loop task flows.

Primary source:
<https://arxiv.org/abs/2406.12045>

Related local normalization source:
[`hal/benchmarks/taubench.py`](../../../repocache/princeton-pli/hal-harness/hal/benchmarks/taubench.py)

## AppWorld

| Item | Value |
| --- | --- |
| Domain | Interactive coding across app APIs and simulated digital workflows |
| Harness-fit | Medium |
| Local-runnability | Heavy |
| Why it matters | Strong long-horizon benchmark for code generation plus environment interaction |

`AppWorld` is the strongest adjacent benchmark in this set for "interactive
coding" rather than pure browser automation or pure customer-service tool use.
Its positioning is especially relevant to `brain`: it exists because simpler
tool-use benchmarks were not enough for agents that must iteratively generate
code and interact with an environment.

That makes it appealing for a later stage where `brain` is benchmarking more
than one narrow loop profile.

Recommendation for `brain`:
keep in the research set, but only pursue after the simpler terminal and SWE
benchmarks are working.

Primary source:
<https://appworld.dev/>

Local source:
[`README.md`](../../../repocache/StonyBrookNLP/appworld/README.md),
[`src/appworld`](../../../repocache/StonyBrookNLP/appworld/src/appworld),
[`experiments`](../../../repocache/StonyBrookNLP/appworld/experiments)
