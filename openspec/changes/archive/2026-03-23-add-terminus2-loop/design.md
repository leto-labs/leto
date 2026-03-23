# Design

## Source of Truth

This change ports Harbor's `src/harbor/agents/terminus_2/terminus_2.py`
control flow into `brain` while adapting it to `brain`'s existing component
boundaries.

The goal is behavioral parity with Terminus-2's loop logic, not reuse of
Harbor's Python runtime shape.

## Architectural Fit

`brain`'s central design decision is composability across five reusable
interfaces: `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport`.

To preserve that:

- `AgentLoop` remains unchanged.
- `brain-core` continues to resolve the selected loop and the registered tool
  registry generically.
- Terminus-2-specific execution semantics are expressed through reusable tool
  and message primitives, not runtime-level special cases.

## Terminal Execution Primitive

Harbor Terminus-2 assumes a persistent terminal session with:

- verbatim keystroke injection
- a caller-supplied minimum wait duration
- timeout reporting with current terminal-state capture
- incremental output polling between iterations

The existing native `shell` tool is insufficient because it is single-shot
`sh -c` execution with no persistent session, no incremental state, and no
keystroke semantics.

This change therefore adds a stateful native terminal-session tool. The loop
uses it as an internal execution primitive by name from the existing runtime
tool registry, while still exposing zero tools to the model during provider
calls.

That preserves runtime composability:

- the runtime still owns a generic tool registry
- loops may choose which registered tools they expose to the provider
- loops may also treat some registered tools as internal execution helpers

## Provider Interaction Model

`Terminus2Loop` uses text-only prompting. It calls the provider with an empty
tool-definition list and expects a structured plain-text JSON response shaped
like Harbor's prompt template:

- `analysis`
- `plan`
- `commands[]`
- optional `task_complete`

The loop parses the response, emits the assistant step, executes the requested
commands through the terminal-session tool, and feeds the resulting terminal
state back as the next user prompt.

## Assistant Reasoning Content

Harbor Terminus-2 optionally supports interleaved thinking by carrying prior
assistant reasoning content into subsequent provider turns.

That behavior belongs in shared message/provider primitives rather than being
encoded as a loop-local side channel. This change therefore extends the common
message model so assistant reasoning content can be preserved and forwarded by
providers that support it.

## Inference Error Classification

Harbor Terminus-2 distinguishes:

- ordinary retryable inference failures
- context-length exhaustion
- output-length exhaustion
- cancellation

The current `BrainError::Inference(String)` shape is too coarse for deterministic
loop recovery. This change adds additive inference failure classification so
loops can decide whether to retry, summarize, or reprompt without relying on
provider-specific string matching as the primary contract.

## Summarization

Harbor Terminus-2 uses a three-subagent summarization handoff:

1. summary generation
2. question generation from the successor agent
3. answer generation from the predecessor agent with fuller context

We will preserve the same logical structure inside `Terminus2Loop`, but it will
be implemented as provider subcalls within the loop rather than as a new runtime
subagent framework.

The loop will:

- trigger proactive summarization when free context falls below a threshold
- trigger fallback summarization on classified context-length failures
- replace older chat history with the handoff prompt while preserving a
  Harbor-compatible ATIF record of the delegation and handoff

## ATIF Strategy

The current ATIF export path is turn-boundary append-only and primarily infers
steps from persisted messages. Terminus-2 needs additional fidelity for:

- handoff/summarization system steps
- user handoff prompt insertion
- completion confirmation turn structure
- assistant reasoning content

This change keeps native ATIF export in `brain-core`, but extends the event/data
path additively so the loop can surface those richer steps without coupling the
runtime to a specific loop implementation.

## Non-Goals

- Reworking `RobustLoop`
- Making all loops use persistent terminals
- Reproducing Harbor's Python `tmux` implementation exactly
- Adding a generic multi-agent orchestration framework beyond what Terminus-2
  needs for summarization
