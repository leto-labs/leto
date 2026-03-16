# Proposal: harden-agent-loop

## Why

brain's `SimpleLoop` is exactly what the name says — simple. It calls the
provider, executes tool calls, re-infers, and loops until done or
`max_iterations` is reached. For a PoC CLI this is fine. For a real coding
agent it's dangerously naive:

1. **Doom loops**: the model can get stuck calling the same tool with the same
   args, burning tokens without making progress. OpenCode, ZeroClaw, and
   pi-mono all have doom-loop detection.

2. **No context compaction**: as the conversation grows, the message history
   eventually exceeds the model's context window. Production agents compact
   (summarize) older messages to stay within limits. OpenCode and pi-mono
   both implement compaction workers.

3. **No retry / error recovery**: if a provider call fails (network error,
   rate limit, 500), SimpleLoop just emits an Error event and stops. A real
   agent should retry with backoff, at least for transient errors.

4. **No sub-agent / delegation**: advanced loops support spawning sub-agents
   for complex tasks (OpenCode's child sessions, Gastown's convoys). This is
   lower priority but the AgentLoop trait should not preclude it.

5. **No steering**: the user can't interrupt a running turn to redirect the
   agent. pi-mono supports steering interrupts and follow-up queues.

## What

### Doom-loop detection

Track the last N tool calls. If the same tool+args pattern repeats more than
a configurable threshold (default 3), emit a warning event and either:
- Inject a system message telling the model it's repeating itself
- Terminate the turn with an error

### Context compaction

When the total token count of the message history exceeds a configurable
threshold (e.g., 80% of the model's context window):
1. Take the oldest N messages (excluding system prompt and the latest turn)
2. Summarize them using a provider call (can use a cheaper/faster model)
3. Replace those messages with a single "summary" message
4. Continue the current turn with the compacted history

This requires the agent loop to be aware of approximate token counts —
either from usage stats in `ChatChunk::Done` or from a tokenizer estimate.

### Retry with backoff

Wrap provider calls in a retry layer:
- Retry on transient errors (network timeout, 429 rate limit, 500/502/503)
- Exponential backoff with jitter
- Configurable max retries (default 3)
- Non-transient errors (400 bad request, 401 auth) fail immediately

### Config extensions

Extend `AgentConfig` with:
- `doom_loop_threshold: u32` (default 3)
- `compaction_threshold: Option<f32>` (fraction of context window, default 0.8)
- `compaction_model: Option<String>` (model to use for summarization)
- `max_retries: u32` (default 3)

### Loop strategies

Keep `SimpleLoop` as-is for backward compatibility. Add a `RobustLoop` (or
enhance `SimpleLoop` with opt-in features) that includes all hardening.

## Change Dependencies

- **Requires**: `enrich-event-model` (emits Retry, Compaction, DoomLoopWarning event variants)
- **Enhances**: `add-config-system` (loop settings in config.toml, but not blocked by it)

## Impact

- **Modifies**: `agent-loop` spec (new config fields, new loop behaviors)
- **Modifies**: `AgentConfig` in `brain-types`
- **New or modified**: `RobustLoop` implementation in `brain-loops`
- **No breaking changes**: SimpleLoop remains; new behavior is opt-in via config
