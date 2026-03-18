# Trace And Result Formats

This page tracks benchmark-adjacent data formats that are worth studying for
`brain`.

The question is not just "how do we score a run?" It is also:

- how do we store runs durably?
- how do we inspect failures?
- how do we export training data?
- how do we compare agent behavior across benchmarks and products?

## Working Conclusion

There is no single format we should adopt wholesale as `brain`'s internal
source of truth. But there are strong ideas worth stealing:

- `ATIF` for cross-agent trajectory interchange
- mini-SWE-agent `.traj.json` and `preds.json` for simple benchmark outputs
- Codex JSONL rollouts for durable production session logs
- OpenHands event logs for backward-compatible per-event persistence
- HAL upload/result JSON for lightweight post-run analysis

The likely `brain` direction is:

1. a `brain`-native rich event log as the source of truth
2. typed tool and mutation schemas as stable control surfaces
3. an `ATIF` exporter for trajectory interchange and training
4. benchmark-specific result exporters such as `preds.json`
5. compact aggregate result files for later analysis

## Format Matrix

| Format | Source | Best use for `brain` | Strength | Main limitation |
| --- | --- | --- | --- | --- |
| `ATIF` | Harbor | Cross-agent trajectory export, training data, replay-friendly analysis | Most complete interchange candidate | More verbose and complex than benchmark-only outputs |
| `.traj.json` | mini-SWE-agent | Simple benchmark/debug trajectory | Very simple and practical | Too benchmark-shaped to be the only runtime log |
| `preds.json` | mini-SWE-agent / SWE-bench | Submission/export format | Minimal and standardized for SWE-bench | Not a trajectory format |
| Response-item streams | Codex / OpenAI-style runtimes | Interoperable turn/event semantics | Good bridge between runtime and provider APIs | Still product-shaped, not a benchmark standard |
| Tool-definition schemas | MCP / OpenHands | Stable tool metadata and policy hints | Strong for control, approvals, analytics | Not sufficient as a full trajectory format |
| Patch / diff formats | `apply_patch`, unified diff | Structured mutation artifacts | Great for validation and replay of edits | Only covers file mutation, not full runs |
| JSONL rollout | Codex | Durable resumable product logs | Strong for real-world runtime state | Product-specific and not a public interchange format |
| Event files | OpenHands SDK | Backward-compatible local event persistence | Good production durability discipline | Less standardized for cross-agent interchange |
| Upload/result JSON | HAL | Aggregate evaluation analytics | Lightweight and analysis-friendly | Not enough detail for replay or training |

## Harbor ATIF

`ATIF` is the most interesting format in this set if the goal is training and
iteration rather than just benchmark submission. Harbor explicitly presents it
as a standardized, JSON-based format for complete agent interaction history,
usable across debugging, visualization, SFT, and RL.

Why it matters:

- captures full interaction history
- includes tool calls and observations
- supports multimodal content
- supports subagent references
- carries token, cost, and logprob-style metrics
- is meant to unify multiple existing agent trace shapes

Harbor's implementation also exposes two especially useful knobs through
`TrajectoryConfig`:

- `raw_content`: preserve raw LLM responses for export and SFT-oriented use
- `linear_history`: split trajectories on summarization boundaries so each file
  stays aligned with what the model actually saw

Those two options are highly relevant to `brain`. They point to a useful
principle:
the runtime log and the training export do not have to be identical views of
the same run.

Recommendation for `brain`:

- do not replace the native event stream with `ATIF`
- do implement an `ATIF` exporter once the event model stabilizes
- keep support for both raw-content and model-visible-history exports

Key local evidence:

- ATIF RFC:
  [`docs/rfcs/0001-trajectory-format.md`](../../repocache/harbor-framework/harbor/docs/rfcs/0001-trajectory-format.md)
- root trajectory model:
  [`src/harbor/models/trajectories/trajectory.py`](../../repocache/harbor-framework/harbor/src/harbor/models/trajectories/trajectory.py)
- trajectory config:
  [`src/harbor/models/agent/trajectory_config.py`](../../repocache/harbor-framework/harbor/src/harbor/models/agent/trajectory_config.py)
- example trajectory:
  [`tests/golden/terminus_2/hello-world-context-summarization.trajectory.json`](../../repocache/harbor-framework/harbor/tests/golden/terminus_2/hello-world-context-summarization.trajectory.json)

Primary source:
<https://harborframework.com/docs/agents/trajectory-format>

## mini-SWE-agent `.traj.json` And `preds.json`

mini-SWE-agent's output formats are interesting because they are small,
explicit, and benchmark-friendly.

The `.traj.json` file stores:

- basic run metadata
- agent/model/environment config
- full message history
- final status and submission

The `preds.json` file stores:

- instance ID
- model name
- generated patch

This is a good reminder that many benchmark workflows do not need a rich trace
format. They need:

- a simple inspectable trajectory
- a minimal submission artifact

Recommendation for `brain`:

- emulate this simplicity for benchmark exporters
- avoid forcing the full production event model into every benchmark artifact

Key local evidence:

- default agent serialization:
  [`src/minisweagent/agents/default.py`](../../repocache/SWE-agent/mini-swe-agent/src/minisweagent/agents/default.py)
- output file docs:
  [`docs/usage/output_files.md`](../../repocache/SWE-agent/mini-swe-agent/docs/usage/output_files.md)

Primary sources:

- <https://mini-swe-agent.com/latest/usage/output_files/>
- <https://mini-swe-agent.com/latest/faq/>

## Codex JSONL Rollouts

Codex is still the strongest reference here for a production-oriented event log.
Its rollout format is append-only JSONL with explicit item variants such as:

- session metadata
- response items
- compaction records
- turn context
- event messages

This is not an interchange standard, but it is a strong example of what a
serious product runtime needs for:

- resumability
- replay
- forking
- compaction-aware recovery
- contextual metadata per turn

Recommendation for `brain`:

- keep a native append-only event log with versioned item types
- preserve turn context and compaction boundaries explicitly
- treat benchmark exports as derived views, not the canonical store

Key local evidence:

- rollout item schema:
  [`protocol/src/protocol.rs`](../../repocache/openai/codex/codex-rs/protocol/src/protocol.rs)

## Response-Item Streams

The Codex/OpenAI-style response stream is worth tracking separately from the
durable log format. It represents a run as typed items such as:

- assistant messages
- function calls
- custom tool calls
- local shell calls
- tool-search calls

Why this matters:

- it separates "what the model asked for" from "how the runtime executed it"
- it is closer to provider-native semantics than a benchmark-specific trace
- it gives `brain` a cleaner boundary between provider output and loop policy

Recommendation for `brain`:

- keep a runtime event model that can represent provider-native response items
- avoid collapsing all model actions into a single opaque text blob
- preserve enough item typing that exporters can reconstruct tool-use intent

Key local evidence:

- Codex response and tool-routing schemas:
  [`protocol/src/protocol.rs`](../../repocache/openai/codex/codex-rs/protocol/src/protocol.rs)
- Codex tool router:
  [`core/src/tools/router.rs`](../../repocache/openai/codex/codex-rs/core/src/tools/router.rs)

## Tool-Definition Schemas

Tool schemas are not full trajectory formats, but they are still critical data
formats for loop design. OpenHands and MCP-style tooling treat tool metadata as
first-class structured data rather than prose:

- title
- read-only hint
- destructive hint
- idempotent hint
- open-world hint

This is important because an agent loop does not just need to log actions. It
also needs to know what kinds of actions are being exposed to the model and
what safety/policy semantics they carry.

Recommendation for `brain`:

- define typed tool metadata early
- keep those semantics separate from prompt text
- use them for approvals, analytics, and future training exports

Key local evidence:

- OpenHands tool annotations:
  [`openhands-sdk/openhands/sdk/tool/tool.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/tool/tool.py)

## Patch And Diff Formats

There is another format family that matters even though it is not a trajectory
format: structured mutation artifacts.

Codex's `apply_patch` flow is a useful reference because it does not treat file
mutation as an arbitrary shell side effect. It reparses and verifies the patch,
derives affected paths, and computes permissions from the parsed patch itself.

Why this matters:

- edits become analyzable artifacts rather than opaque shell behavior
- mutation intent can be reviewed separately from execution logs
- permission systems can reason about exact affected paths
- replay and benchmarking become cleaner because patch output is explicit

Recommendation for `brain`:

- keep unified diff or `apply_patch`-style edits as first-class artifacts
- prefer structured edit surfaces over "sed in bash" for tracked mutations
- still allow shell-based editing, but do not make it the only mutation path

Key local evidence:

- Codex `apply_patch` handler:
  [`core/src/tools/handlers/apply_patch.rs`](../../repocache/openai/codex/codex-rs/core/src/tools/handlers/apply_patch.rs)

## OpenHands Event Logs

OpenHands is useful as a durability and compatibility reference. Its SDK keeps
events in persistent per-event JSON files behind an `EventLog` abstraction with
locking, and its internal guidance explicitly says old events must continue to
load.

That is a strong design signal:

- trace schemas become long-lived compatibility surfaces
- backward compatibility matters even if the loop itself evolves quickly

Recommendation for `brain`:

- add explicit event-schema versioning before the trace format spreads
- avoid assuming event storage is disposable internal detail

Key local evidence:

- event log implementation:
  [`openhands-sdk/openhands/sdk/conversation/event_store.py`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/conversation/event_store.py)
- SDK compatibility guidance:
  [`openhands-sdk/openhands/sdk/AGENTS.md`](../../repocache/OpenHands/software-agent-sdk/openhands-sdk/openhands/sdk/AGENTS.md)

## HAL Result Upload JSON

HAL's upload/result JSON is not a trajectory format, but it is useful for how
to structure benchmark analysis outputs. The fixture shape includes:

- `metadata`
- `config`
- aggregate `results`
- `raw_eval_results`
- `raw_logging_results`

The loader then aggressively reduces those raw fields into minimal data needed
for analysis, such as rewards, action names, token totals, and latencies.

This is a strong pattern for `brain`:

- keep rich raw traces
- derive compact analysis artifacts separately
- do not force analytics code to hold full conversations in memory

Key local evidence:

- example upload payload:
  [`tests/reliability_eval/fixtures/results/taubench_airline/taubench_toolcalling_gpt_4o_mini_baseline_r0/taubench_toolcalling_gpt_4o_mini_baseline_r0_UPLOAD.json`](../../repocache/princeton-pli/hal-harness/tests/reliability_eval/fixtures/results/taubench_airline/taubench_toolcalling_gpt_4o_mini_baseline_r0/taubench_toolcalling_gpt_4o_mini_baseline_r0_UPLOAD.json)
- result loader:
  [`reliability_eval/loaders/results.py`](../../repocache/princeton-pli/hal-harness/reliability_eval/loaders/results.py)

## Implications For `brain`

The strongest format design for `brain` looks like a layered model:

### 1. Native runtime log

Append-only, versioned, rich enough for replay and resume.

Must include:

- turn boundaries
- tool calls and results
- timing and cost
- compaction or condensation events
- user steering / interruption events
- enough context metadata to reproduce the run shape

### 2. Tool and mutation schemas

Separate, typed representations for:

- tool definitions
- tool annotations
- patch or diff artifacts
- approval-relevant mutation metadata

These are not replacements for runtime logs. They are control surfaces.

### 3. Interchange trajectory export

Likely `ATIF`-shaped or directly `ATIF`-compatible.

Must support:

- linear model-visible trajectories
- raw-response export for training
- optional multimodal and subagent references

### 4. Benchmark submission exports

Examples:

- `preds.json` for SWE-bench-style workflows
- benchmark-specific result JSON for external evaluators

### 5. Aggregate analytics artifacts

Small files optimized for:

- dashboarding
- parity analysis
- cost/latency comparisons
- reliability studies

The key idea is to avoid choosing one format to do all four jobs.
