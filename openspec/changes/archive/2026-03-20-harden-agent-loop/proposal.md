# Proposal: harden-agent-loop

## Why

brain's `SimpleLoop` is exactly what the name says — simple. It calls the
provider, executes tool calls, re-infers, and loops until done or
`max_iterations` is reached. For a PoC CLI this is fine. For a real coding
agent it's dangerously naive:

1. **Doom loops**: the model can get stuck calling the same tool with the same
   args, burning tokens without making progress.

2. **No context compaction**: as the conversation grows, the message history
   eventually exceeds the model's context window. The runtime needs a way to
   summarize older context inside a turn so the model can continue operating.

3. **No retry / error recovery**: if a provider call fails (network error,
   rate limit, 500), SimpleLoop just emits an Error event and stops. A real
   agent should retry with backoff, at least for transient errors.

4. **No steering**: when the model falls into a repetitive tool loop, the loop
   needs a way to inject corrective guidance before giving up.

## What

### Doom-loop detection

Track repeated tool signatures. If the same tool+args pattern repeats more than
a configurable threshold (default 3), emit a warning event and either:
- Inject a system message telling the model it's repeating itself
- Terminate the turn with an error, depending on strategy

### Context compaction

When the estimated token count of the in-memory conversation exceeds a
configurable fraction of the model's context window:
1. Take the oldest N messages (excluding system prompt and the latest turn)
2. Summarize them using a provider call (optionally with a different model)
3. Replace those messages with a single "summary" message
4. Continue the current turn with the compacted history

This compaction is primarily a turn-level runtime behavior. It need not rewrite
stored session history in v1.

### Retry with backoff

Wrap provider calls in a retry layer:
- Retry on transient errors (network timeout, 429 rate limit, 500/502/503)
- Exponential backoff with jitter
- Configurable max retries (default 3)
- Non-transient errors (400 bad request, 401 auth) fail immediately

### Config extensions

Extend `AgentConfig` with:
- `doom_loop_threshold: u32` (default 3)
- `doom_loop_strategy: steer | error` (default `steer`)
- `compaction_threshold: Option<f32>` (fraction of context window, default 0.8)
- `compaction_model: Option<String>` (model to use for summarization)
- `max_retries: u32` (default 3)

### Loop strategies

Keep `SimpleLoop` as-is for backward compatibility. Add a `RobustLoop` that
includes retries, doom-loop mitigation, and compaction.

## Change Dependencies

- **Coordinates with**: `enrich-event-model` (the required events already exist)
- **Enhances**: `add-config-system` (loop settings in config.toml)

## Impact

- **Modifies**: `agent-loop` spec (new config fields, new loop behaviors)
- **Modifies**: `AgentConfig` in `brain-types`
- **New or modified**: `RobustLoop` implementation in `brain-loops`
- **No breaking changes**: SimpleLoop remains; new behavior is opt-in via config
