# Loop Design Notes

This page connects the benchmark research to `brain`'s next `AgentLoop`
iteration.

The main tension is straightforward:

- benchmark evidence often rewards simpler harnesses
- production coding agents still end up more complex for real reasons

The goal for `brain` should not be to pick one side. It should be to separate:

- benchmark-effective simplicity
- product-driven complexity
- and complexity that is merely accidental

## What mini-SWE-agent Actually Proves

mini-SWE-agent is strong evidence for a narrower claim than "simpler is
better."

What it really shows is:

- benchmark harness variance matters a lot
- simpler control flow can remove many accidental failure modes
- a bash-centric loop can already get strong coding-benchmark results
- a small scaffold is useful when the goal is comparing models rather than
  maximizing runtime capability

The mini FAQ is explicit about this positioning. It recommends `mini` when you
want:

- a quick command-line tool
- a very simple control flow
- simpler and more stable sandboxing and benchmark evaluations
- FT or RL workflows that avoid overfitting to a specific scaffold

The same FAQ is equally explicit about when `mini` is not enough:

- if you need specific tools
- if you want to experiment with different history processors
- if you want more flexible configuration and runtime behavior

That is the key nuance for `brain`: mini is excellent evidence for a baseline
benchmark harness, not a proof that bash-only and linear history are the best
long-term agent design.

Key evidence:

- FAQ:
  <https://mini-swe-agent.com/latest/faq/>
- README:
  [`README.md`](../../repocache/SWE-agent/mini-swe-agent/README.md)
- default agent:
  [`src/minisweagent/agents/default.py`](../../repocache/SWE-agent/mini-swe-agent/src/minisweagent/agents/default.py)

## Why Simple Often Wins On Benchmarks

mini-SWE-agent is the clearest current argument for simplicity:

- bash as the only tool
- completely linear history
- stateless command execution via `subprocess.run`
- explicit step and cost limits
- simple JSON trajectory export

Its default agent implementation reinforces the same point in code:

- `step()` is query then execute
- every loop appends messages linearly
- the serialized trajectory is just config, messages, and final status

That simplicity is not just aesthetic. It reduces entire classes of failure:

- stateful shell drift
- shell prompt / termination heuristics
- tool-routing bugs
- persistent-session corruption
- framework-specific hidden behavior

The benchmark-side reason this matters is straightforward: the simpler harness
removes more non-model variance. On SWE-bench-style tasks, that can be a major
advantage because the benchmark mostly cares about repository exploration,
patch generation, and passing tests, not about interactive UX richness.

Key evidence:

- default agent:
  [`src/minisweagent/agents/default.py`](../../repocache/SWE-agent/mini-swe-agent/src/minisweagent/agents/default.py)
- output files:
  [`docs/usage/output_files.md`](../../repocache/SWE-agent/mini-swe-agent/docs/usage/output_files.md)

## Why "Just Bash" Is Probably Not Optimal Long Term

The strongest argument against over-generalizing from mini is not aesthetic. It
is that pure bash loops give up important advantages once the environment stops
looking like a clean software-engineering benchmark.

### 1. Structured tools can be more context-efficient than bash

Raw shell output is often a bad context representation. Product runtimes add
typed file/search/edit tools and output-truncation logic because dumping full
terminal output back into the prompt scales badly.

OpenCode is explicit about this. Oversized tool output is spilled to a file,
and the agent is told to use targeted tools like `Task`, `Grep`, or paged
`Read` rather than reading the whole output back into context.

Implication for `brain`:
context preservation matters, but the best way to preserve useful context is
often structured summaries or typed tool results, not full shell transcripts.

There is a deeper design point here:
bash tends to encode intent, arguments, mutation target, and result shape all
inside one text channel. Typed tools separate those concerns. That separation
is useful not only for safety and analytics, but also for training data,
because the action surface becomes legible instead of being buried in command
strings.

Key evidence:

- truncation layer:
  [`packages/opencode/src/tool/truncation.ts`](../../repocache/anomalyco/opencode/packages/opencode/src/tool/truncation.ts)

### 2. Typed tools provide stronger control surfaces

Codex and OpenHands both show why structured tools matter beyond convenience.

Codex routes tools through a typed registry and validates `apply_patch` before
execution, computing affected file paths and permissions from the patch itself.
OpenHands tool definitions carry semantic hints like read-only, destructive,
idempotent, and open-world behavior.

Implication for `brain`:
typed tools are not just benchmark sugar. They enable:

- better permission models
- more precise approvals
- better replay semantics
- cleaner analytics
- safer mutation flows

They also provide better supervision surfaces. A trajectory that says
"call `file_read` on path X" or "apply this patch to paths Y/Z" is much easier
to learn from, filter, or compare than a trajectory where those semantics are
only implicit in a shell command.

Key evidence:

- Codex tool routing:
  [`core/src/tools/router.rs`](../../repocache/openai/codex/codex-rs/core/src/tools/router.rs)
- Codex apply-patch handler:
  [`core/src/tools/handlers/apply_patch.rs`](../../repocache/openai/codex/codex-rs/core/src/tools/handlers/apply_patch.rs)
- OpenHands tool definitions:
  [`openhands-sdk/openhands/sdk/tool/tool.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/tool/tool.py)

### 3. Stateless shell execution trades away real capability

mini's `subprocess.run` approach is a real robustness win. But it also gives up
stateful shell workflows:

- persistent cwd changes
- exported environment state across steps
- long-running processes
- REPL-style interaction
- terminal UIs and richer PTY flows

Those are not niche product details. They matter in real coding-agent work and
in benchmarks like `Terminal-Bench`.

### 4. Pure bash does not map well to richer client capabilities

ACP, IDE shells, MCP-connected runtimes, and modern coding agents increasingly
expose capabilities that are not best represented as pasted shell commands:

- editor-native file reads and writes
- permission prompts
- structured patch application
- resource browsing
- subagent control
- terminal streaming as a separate channel

If `brain` wants to exploit ACP or editor-owned capabilities well, it probably
cannot stay at "everything is bash" forever.

### 5. Long-horizon agents need more than linear append-only history

Linear history is extremely appealing for debugging and evaluation. But longer
interactive sessions eventually need:

- compaction or condensation
- truncation policies
- resumable state
- steering interruptions
- subagent references

The important lesson from Harbor is especially useful here:
`linear_history` can be an export mode even when the runtime itself needs
compaction-aware internal state.

Key evidence:

- trajectory config:
  [`src/harbor/models/agent/trajectory_config.py`](../../repocache/harbor-framework/harbor/src/harbor/models/agent/trajectory_config.py)

## Where Additional Complexity Pays For Itself

The right question is not "why is Codex more complex than mini?" It is "which
parts of that complexity buy a real capability or reliability improvement?"

The source survey suggests several categories where the answer is "yes":

- typed tools and mutation formats improve control, approvals, and analytics
- truncation and compaction reduce prompt waste rather than adding capability
- persistent event logs support resumability and postmortem debugging
- stuck detection targets a real long-horizon failure mode
- stateful shells and PTYs unlock workflows a stateless subprocess loop cannot

This implies a more disciplined interpretation of benchmark simplicity:

- simplicity is good when it removes confounders
- complexity is justified when it buys a measurable capability or stability gain
- the dangerous category is accidental complexity that does neither

Key evidence:

- OpenCode truncation:
  [`packages/opencode/src/tool/truncation.ts`](../../repocache/anomalyco/opencode/packages/opencode/src/tool/truncation.ts)
- OpenHands stuck detector:
  [`openhands-sdk/openhands/sdk/conversation/stuck_detector.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/conversation/stuck_detector.py)
- OpenHands event log:
  [`openhands-sdk/openhands/sdk/conversation/event_store.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/conversation/event_store.py)
- Codex `apply_patch` handler:
  [`core/src/tools/handlers/apply_patch.rs`](../../repocache/openai/codex/codex-rs/core/src/tools/handlers/apply_patch.rs)

## Why Codex-Class Runtimes Are More Complex

Codex, OpenCode, and OpenHands are not obviously "more correct" just because
they are more complex. But a large part of their complexity is solving real
product constraints that benchmark loops can dodge.

Those constraints include:

- stateful shell or PTY interaction
- streamed partial output
- approval and sandbox policies
- resumable sessions and durable logs
- context compaction or condensation
- mid-turn steering and user interrupts
- subagents and delegation
- richer tool sets including MCP and editor-owned capabilities
- backward-compatible persistence over long-lived sessions

This is why Codex's JSONL rollout and OpenHands' persistent event log matter.
They are not benchmark optimizations first; they are runtime survivability
mechanisms for interactive use.

OpenCode adds another important point: some complexity is about preventing
context waste, not adding it. Its truncation layer and compaction path exist
because raw tool output and long transcripts are bad long-run prompt shapes.

Key evidence:

- Codex rollout items:
  [`protocol/src/protocol.rs`](../../repocache/openai/codex/codex-rs/protocol/src/protocol.rs)
- Codex competitor analysis:
  [`docs/competitors/Codex.md`](../competitors/Codex.md)
- OpenCode competitor analysis:
  [`docs/competitors/OpenCode.md`](../competitors/OpenCode.md)
- Cline competitor analysis:
  [`docs/competitors/Cline.md`](../competitors/Cline.md)
- OpenHands event log:
  [`openhands-sdk/openhands/sdk/conversation/event_store.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/conversation/event_store.py)
- OpenHands stuck detection:
  [`openhands-sdk/openhands/sdk/conversation/stuck_detector.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/conversation/stuck_detector.py)

## What Existing Result Data Suggests

The benchmark infrastructure we studied points to a useful split between
runtime traces and evaluation artifacts.

### Harbor / Terminal-Bench

Harbor stores:

- job config and results
- per-trial config and results
- agent trajectory
- verifier outputs

This suggests `brain` should preserve both:

- the agent-side trace
- the verifier-facing result surface

### HAL

HAL's result loaders explicitly strip full raw logs down to minimal fields for
analysis. That is a strong hint that analytics should not depend on replaying
entire conversations.

### mini-SWE-agent

mini separates:

- the full trajectory for inspection
- the tiny `preds.json` submission artifact for scoring

That is probably the right default for `brain` too.

But mini's result format should not be misread as a full runtime design target.
It is excellent for evaluation ergonomics because it deliberately omits large
classes of interactive-runtime concerns.

## Hypotheses `brain` Should Actually Test

The right response is not to argue abstractly about simplicity. It is to run
ablations.

### Hypothesis 1: bash-only is the strongest benchmark baseline, not the best general loop

Compare:

- `brain` eval profile with bash-only execution
- `brain` eval profile with file/search/edit/patch tools enabled

Measure:

- success rate
- cost
- prompt growth
- tool/output token volume
- failure modes

### Hypothesis 2: structured tools reduce context pressure on long tasks

Compare:

- shell-heavy exploration
- typed search/read/edit/patch flows

Measure:

- average prompt size over time
- compaction frequency
- token cost per successful task
- number of large-output truncation events

### Hypothesis 3: stateless shell is robust but caps capability

Compare:

- `subprocess.run`-style stateless execution
- PTY/stateful shell execution

Measure:

- reliability on SWE-bench-like tasks
- ability to complete Terminal-Bench tasks involving persistent process state
- shell/session failure rate

### Hypothesis 4: linear history is best as an export view, not always as runtime state

Compare:

- fully linear prompt history
- runtime compaction with linear-history export

Measure:

- benchmark success
- resume fidelity
- model-visible context size
- quality of exported trajectories for training

### Hypothesis 5: typed tools beat bash when the task depends on control, not just reachability

Compare:

- bash-only editing and search
- typed read/search/edit/patch tools with shell kept as fallback

Measure:

- approval precision
- replayability of actions
- number of malformed or ambiguous edit attempts
- context used per successful repository-navigation step

### Hypothesis 6: richer event state improves recovery and debugging even if it does not raise raw benchmark scores

Compare:

- minimal trajectory-only persistence
- append-only event log with compaction and interruption markers

Measure:

- resume success after interruption
- ability to explain failures after the fact
- cost of deriving benchmark exports and analytics
- operator effort to debug bad runs

## Design Implications For `brain`

### 1. Keep a minimal evaluation path

`brain` should have a loop profile optimized for benchmark comparability and
operational stability.

Properties:

- linear history by default
- explicit max-step and cost budgets
- minimal tool set
- no unnecessary product features in the hot path
- deterministic export artifacts

But "minimal tool set" should mean "minimal set needed for the experiment," not
"bash only forever."

This could be a dedicated `EvalLoop`, or it could be a tightly configured
profile of a richer loop implementation. The important part is behavioral
discipline, not the type name.

### 1a. Do not confuse "minimal" with "opaque"

A minimal evaluation path should still preserve:

- typed events
- explicit tool identities
- explicit mutation artifacts
- explicit cost and budget data

What should be minimal is runtime behavior, not observability.

### 2. Keep product complexity modular

Interactive/runtime features should remain opt-in layers:

- retries
- doom-loop detection
- compaction
- approvals
- stateful shell support
- steering
- subagents

These features are useful, but they should not make the default benchmark path
fragile.

### 3. Separate source-of-truth events from exports

`brain` should not force one trace shape to serve every consumer.

Prefer:

- rich internal event stream
- replay/resume log
- benchmark submission exporters
- interchange trajectory exporters
- compact analytics artifacts

This is the cleanest way to preserve mini-style benchmark simplicity without
freezing the whole runtime around mini's assumptions.

### 3a. Treat shell as one tool family, not the whole ontology

The long-term design risk in "just bash" is not only safety. It is that shell
becomes the only representation of agent action. That makes traces harder to:

- compare across runtimes
- train on
- replay precisely
- govern with permissions
- summarize compactly

`brain` should keep shell execution available, but it should not force file
reads, file edits, search, and patch application to masquerade as raw shell
strings when better typed surfaces exist.

### 4. Make evaluation mode a first-class use case

The benchmark docs now make it clear that we care deeply about running `brain`,
Codex, Claude Code, and OpenCode under the same benchmarks. That means
evaluation mode should be designed intentionally, not as an afterthought.

Evaluation mode should make it easy to:

- hold model and budget constant when possible
- record when model parity is only approximate
- disable non-essential product features
- emit comparable results and traces

## Recommended Near-Term Loop Shape

For `brain`, the next loop should probably be "simple plus hardening," not
"full Codex clone."

That means:

- preserve the simple core request -> tool -> re-request loop
- keep a benchmark-focused profile that can stay close to bash-only when useful
- avoid assuming that bash-only is the final product architecture
- add doom-loop detection
- add retry with backoff
- add compaction only when clearly needed, and keep linear-history exports
- add structured trace/export support early
- add a small set of typed high-value tools early, especially search/read/edit
  or patch flows
- keep shell support as a fallback and escape hatch, not the only tool surface
- avoid coupling the loop to UI-specific concerns

In other words:
mini-SWE-agent is a strong benchmark-design reference, while Codex is a strong
interactive-runtime reference. `brain` should learn from both without becoming
either one.

## Working Recommendation

Use benchmark evidence to keep the core loop honest:

- simple
- inspectable
- stable
- exportable

But do not treat benchmark-effective simplicity as a universal design law. The
better working rule is:

- use simplicity to remove unnecessary variance
- add typed capabilities when they buy real control, safety, or context
  efficiency
- validate those additions with ablations instead of product intuition alone

Use product-runtime evidence to decide which additional layers must exist, but
keep them modular enough that benchmark runs can stay as simple as possible.
