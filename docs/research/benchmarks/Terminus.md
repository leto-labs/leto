# Terminus

This page studies the `Terminus` family as a concrete set of agent-loop designs
worth re-implementing in `brain`, not just as leaderboard competitors.

The important takeaway is not "KIRA added a few tricks." It is that the
benchmark evidence and source diff both point to a stronger claim: meaningful
Terminal-Bench gains can come from redesigning the loop contract, terminal
execution model, and completion policy without changing the benchmark itself.

## Why Terminus Matters To `brain`

`Terminus` is especially relevant for this repo for three reasons:

- Harbor ships [`terminus-2`](../../../repocache/harbor-framework/harbor) as a
  first-party built-in agent, so it is a real comparison surface we can run
  locally.
- `Terminus 2` is clearly loop-centric rather than product-shell-centric: the
  core design is a stateful terminal session, a planner/executor loop, and a
  prompt/parser contract.
- KRAFTON's `Terminus-KIRA` is a visible proof that large gains can come from
  harness and loop redesign, not only from changing models.

For `brain`, that makes Terminus more useful than a closed proprietary agent.
It gives us a family of designs we can actually inspect and selectively
re-implement.

## Leaderboard Snapshot

As observed on 2026-03-22, the live `terminal-bench@2.0` leaderboard shows a
large gap between `Terminus 2` and `Terminus-KIRA`.

Useful apples-to-near-apples comparisons on the public leaderboard:

- `Terminus-KIRA` with `Gemini 3.1 Pro`: `74.8%`
- `Terminus 2` with `Gemini 3 Pro`: `56.9%`
- `Terminus-KIRA` with `Claude Opus 4.6`: `74.7%`
- `Terminus 2` with `Claude Opus 4.6`: `62.9%`
- current best `Terminus 2` entry overall in the observed snapshot:
  `GPT-5.3-Codex` at `64.7%`

That does not prove the entire delta is loop quality alone. Model families,
dates, and implementation maturity still matter. But it is strong evidence that
the Terminus design space contains more headroom than the stock Harbor
implementation.

Primary source:

- Terminal-Bench leaderboard:
  <https://www.tbench.ai/leaderboard/terminal-bench/2.0>

## Terminus 2 Design

Harbor's `Terminus2` is a terminal-native, stateful agent loop built around a
`tmux` session. The key pieces are:

- a persistent terminal session via
  [`tmux_session.py`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/tmux_session.py)
- a planner/executor prompt loop in
  [`terminus_2.py`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/terminus_2.py)
- parser-driven structured output, either JSON or XML
- explicit completion confirmation
- aggressive summarization and trajectory support for long runs

### Core Contract

The model is not called through native tool definitions. Instead, it is told to
emit a structured response envelope containing:

- `analysis`
- `plan`
- `commands`
- optional `task_complete`

The default JSON prompt in
[`terminus-json-plain.txt`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/templates/terminus-json-plain.txt)
is effectively an instruction-level tool protocol: the model must serialize its
action plan as prompt-constrained JSON, which Harbor then parses back into
commands.

### Strengths

`Terminus 2` gets several important things right:

- It uses a real stateful terminal instead of stateless subprocess calls.
- It forces the model to externalize both `analysis` and `plan`, which is good
  for debugging and trajectory inspection.
- It already has completion double-check logic.
- It invests heavily in summarization, trajectory export, and long-horizon run
  handling.

This makes it a credible benchmark harness rather than a toy agent.

### Limitations

The main weaknesses visible in the source are also straightforward:

- The parser contract is prompt-heavy and brittle. The default JSON template is
  `54` lines long and spends a lot of prompt budget teaching the model how to
  format actions.
- Parser-driven structure is weaker than native tool calling because the model
  can still drift, over-explain, or emit malformed structure.
- The command execution contract depends on guessed durations and repeated
  polling rather than stronger completion signals.
- The multimodal story is weak in the base loop.
- Completion confidence is still largely prompt-mediated rather than enforced
  through a richer control surface.

For `brain`, the interesting point is that these are loop design issues, not
just prompt wording issues.

## Terminus-KIRA

`Terminus-KIRA` is explicitly presented by KRAFTON AI as "a smarter agent
harness for Terminal-Bench, built on Terminus 2." In code, it is a real loop
redesign, not a cosmetic fork.

Primary sources:

- [`README.md`](../../../repocache/krafton-ai/KIRA/README.md)
- [`terminus_kira.py`](../../../repocache/krafton-ai/KIRA/terminus_kira/terminus_kira.py)
- [`terminus-kira.txt`](../../../repocache/krafton-ai/KIRA/prompt-templates/terminus-kira.txt)

### What Changes

KIRA keeps the broad terminal-first shape of Terminus 2, but changes several
core mechanisms:

- It subclasses `Terminus2` but replaces parser-driven ICL output with native
  LLM tool calling.
- It defines a small typed tool surface:
  - `execute_commands`
  - `task_complete`
  - `image_read`
- It adds marker-based polling for command completion.
- It adds multimodal image analysis.
- It adds prompt caching for Anthropic-backed runs.
- It strengthens completion verification before accepting `task_complete`.

This is not just "better prompt engineering." It changes the model/runtime
boundary.

### Native Tool Calling Instead Of Prompt Parsing

This is the most important delta.

In `Terminus 2`, the prompt itself encodes the action protocol. In `KIRA`, the
action protocol is encoded as actual tool definitions passed to the model.

That has several effects:

- the system prompt becomes much shorter
- structure is enforced by the model API rather than by post-hoc parsing
- the tool surface becomes explicit and inspectable
- the loop no longer burns as much context budget on response-format coaching

The prompt length difference is a useful proxy:

- `Terminus 2` JSON template:
  [`terminus-json-plain.txt`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/templates/terminus-json-plain.txt)
  is `54` lines
- `Terminus-KIRA` template:
  [`terminus-kira.txt`](../../../repocache/krafton-ai/KIRA/prompt-templates/terminus-kira.txt)
  is `11` lines

That is a meaningful design change, not a tiny tweak.

### Marker-Based Polling

KIRA improves terminal execution semantics by appending a unique marker after
each command and checking for early completion rather than always sleeping for
the full requested duration.

This matters because it changes the temporal behavior of the loop:

- fast commands do not waste as much idle time
- the loop can react sooner to command completion
- the model gets feedback earlier without having to over-wait

For `brain`, this is one of the strongest concrete ideas to borrow. It is a
runtime execution strategy, not merely a reasoning policy.

### Stronger Completion Verification

KIRA retains the Terminus completion-confirmation idea but pushes it further.
Its prompt and README both emphasize:

- re-reading the original task before `task_complete`
- explicitly identifying the minimum required file/system changes
- checking that no extra side effects were left behind
- using multiple QA perspectives during completion review

This is exactly the sort of benchmark-facing discipline our own Harbor runs
suggest we need, especially on tasks where the agent diagnoses correctly but
stops too early or leaves unnecessary mutations behind.

### Multimodal Support

`image_read` is the other important addition. It gives the loop a native way to
bring visual artifacts back into the reasoning process.

That matters because a terminal-first agent is not always a text-only agent.
Tasks like chess boards, screenshots, charts, and diagrams are all awkward if
the loop assumes shell output is the only observation channel.

## KiraClaw As The Second Iteration

The broader `KIRA` repository also contains `KiraClaw`, which is not another
Terminal-Bench harness. It is a broader product/runtime redesign and should be
read as the next architectural step after benchmark-oriented Terminus-KIRA.

Primary sources:

- [`KiraClaw/docs/architecture.md`](../../../repocache/krafton-ai/KIRA/KiraClaw/docs/architecture.md)
- [`KiraClaw/README.md`](../../../repocache/krafton-ai/KIRA/KiraClaw/README.md)

### What Changes In KiraClaw

KiraClaw shifts the focus from benchmark harness improvements to a longer-lived
embedded agent runtime:

- one long-running gateway per host
- embedded engine rather than a CLI subprocess wrapper
- serialized runs per session
- explicit `speak` versus internal summary
- channel adapters on one daemon boundary
- generous turn and timeout limits for real work

This matters because it suggests a second lesson beyond the benchmark harness
itself:

- `Terminus-KIRA` is a better benchmark loop
- `KiraClaw` is a better product/runtime architecture

Those are related, but they are not the same design problem.

### Why This Matters To `brain`

`brain` already separates `Provider`, `Tool`, `Store`, `Transport`, and
`AgentLoop`. That gives us a good seam for experimenting with alternate loops.
But KiraClaw is also a reminder that some improvements do not belong only in
`AgentLoop`.

Several of the most meaningful Terminus-family gains sit at the boundary
between:

- loop policy
- tool/control surface
- terminal execution runtime
- completion semantics

So the right conclusion is not "copy Terminus into one more loop trait
implementation." It is:

- we should prototype multiple loop families
- and we should be willing to redesign the runtime contract around them

## Candidate Loop Families To Re-Implement

The Terminus family suggests at least four concrete re-implementation targets
for `brain`.

### 1. Parser-Driven Terminal Loop

This is the stock Terminus 2 pattern:

- stateful terminal session
- explicit `analysis` and `plan`
- prompt-constrained command envelope
- completion confirmation

Why build it:

- good benchmark baseline
- easy to compare against Harbor Terminus 2 directly
- useful for understanding how much parser-mediated structure hurts or helps

### 2. Native-Tool Terminal Loop

This is the Terminus-KIRA pattern:

- stateful terminal session
- native tool calls rather than prompt-parsed JSON/XML
- short prompt
- explicit command tool plus completion tool

Why build it:

- strongest immediate design contrast with our current robust loop
- likely the highest-value direct experiment
- cleaner action surface for tracing and later training

### 3. Marker-Polling / Terminal-Optimized Execution Loop

This isolates the runtime/execution insight from KIRA:

- command markers
- early completion detection
- bounded wait/poll strategy
- better long-command visibility

Why build it:

- directly addresses our current long-step and timeout pathologies
- may improve both latency and robustness without changing model reasoning

### 4. Embedded Long-Running Runtime

This is the KiraClaw-style direction:

- stable daemon/session boundary
- serialized runs
- explicit outward speech vs internal summary
- generous long-running task policy

Why build it:

- more relevant for product architecture than for leaderboard work
- still useful if we want `brain` to graduate from benchmark harness to
  durable local runtime

## Working Conclusion

For `brain`, the Terminus lesson is broader than "raise iteration limits" or
"improve completion prompts."

The stronger conclusion is:

- benchmark performance can move materially when the loop contract changes
- the most interesting changes are architectural, not cosmetic
- we should test alternate loop families directly instead of only patching the
  current robust loop in place

That means the next design phase should consider deliberate re-implementation
tracks such as:

- a Terminus-2-style parser loop
- a Terminus-KIRA-style native-tool loop
- a marker-based terminal execution layer
- eventually, a KiraClaw-style embedded runtime boundary

If we do that, our benchmark work becomes a real loop-design program rather
than a series of isolated bug fixes.

## Primary Sources

- Live leaderboard:
  <https://www.tbench.ai/leaderboard/terminal-bench/2.0>
- Harbor Terminus 2:
  [`terminus_2.py`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/terminus_2.py)
- Terminus 2 JSON prompt:
  [`terminus-json-plain.txt`](../../../repocache/harbor-framework/harbor/src/harbor/agents/terminus_2/templates/terminus-json-plain.txt)
- Terminus-KIRA README:
  [`README.md`](../../../repocache/krafton-ai/KIRA/README.md)
- Terminus-KIRA implementation:
  [`terminus_kira.py`](../../../repocache/krafton-ai/KIRA/terminus_kira/terminus_kira.py)
- Terminus-KIRA prompt:
  [`terminus-kira.txt`](../../../repocache/krafton-ai/KIRA/prompt-templates/terminus-kira.txt)
- KiraClaw architecture:
  [`architecture.md`](../../../repocache/krafton-ai/KIRA/KiraClaw/docs/architecture.md)
