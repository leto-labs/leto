# Tasks: harden-agent-loop

## Implementation Checklist

- [ ] Add doom-loop detection fields to AgentConfig (threshold, strategy)
- [ ] Implement doom-loop detection: track tool call history, detect repetition
- [ ] Implement doom-loop mitigation: inject system message or terminate turn
- [ ] Add compaction config fields to AgentConfig (threshold, model)
- [ ] Implement token counting / estimation (from usage stats or tokenizer)
- [ ] Implement context compaction: select messages, summarize, replace
- [ ] Add retry config fields to AgentConfig (max_retries, backoff params)
- [ ] Implement retry wrapper for provider calls with exponential backoff + jitter
- [ ] Classify errors as transient (429, 5xx, network) vs permanent (400, 401)
- [ ] Create `RobustLoop` that composes all hardening features
- [ ] Ensure `SimpleLoop` remains unchanged for backward compatibility
- [ ] Add new Event variants if needed (e.g., `Compaction`, `Retry`, `DoomLoopWarning`)
- [ ] Write unit tests for doom-loop detection
- [ ] Write unit tests for compaction trigger logic
- [ ] Write unit tests for retry with backoff
- [ ] Write integration test: RobustLoop with MockProvider that triggers retries
