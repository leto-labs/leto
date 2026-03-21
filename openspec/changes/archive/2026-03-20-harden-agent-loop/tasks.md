# Tasks: harden-agent-loop

## Implementation Checklist

- [x] Add doom-loop detection fields to AgentConfig (threshold, strategy)
- [x] Implement doom-loop detection: track repeated tool signatures
- [x] Implement doom-loop mitigation: inject a steering message or terminate the turn
- [x] Add compaction config fields to AgentConfig (threshold, model)
- [x] Implement token estimation using approximate text sizing and model context limits
- [x] Implement turn-level context compaction: select messages, summarize, replace
- [x] Add retry config fields to AgentConfig (`max_retries`)
- [x] Implement retry wrapper for provider calls with exponential backoff + deterministic jitter
- [x] Classify errors as transient (429, 5xx, network/timeout) vs permanent
- [x] Create `RobustLoop` that composes retries, compaction, and doom-loop mitigation
- [x] Ensure `SimpleLoop` remains unchanged for backward compatibility
- [x] Reuse existing `Retry`, `Compaction`, and `DoomLoopWarning` events
- [x] Write unit tests for doom-loop detection
- [x] Write unit tests for compaction trigger logic
- [x] Write unit tests for retry with backoff
- [x] Register `RobustLoop` in the default ACP and CLI runtimes
