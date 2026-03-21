# Design: harden-agent-loop

## Decisions

- Add a separate `RobustLoop` instead of changing `SimpleLoop` in place.
- Keep the `AgentLoop` trait unchanged.
- Keep hardening behavior configurable through `AgentConfig`.
- Retry only while provider failure happens before visible output for the current
  attempt, to avoid duplicated streamed tokens.
- Perform compaction inside the current turn by replacing older in-memory
  messages with a generated summary message.

## Notes

- Doom-loop mitigation uses repeated tool name + serialized arguments as the
  signature.
- Default doom-loop behavior is `steer`, with escalation to an error if the
  repetition continues after steering.
- Compaction uses model context limits when available from provider metadata and
  falls back to doing nothing when no context limit is known.
