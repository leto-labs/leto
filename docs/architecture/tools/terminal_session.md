# Terminal Session Tool

## Purpose

| Tool | Role |
| --- | --- |
| `terminal_session` | Interact with a persistent terminal using verbatim keystrokes and bounded capture |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Current implementation |

## Actions

| Action | Meaning |
| --- | --- |
| `send_keys` | Send keystrokes or logical keys into a persistent session |
| `capture` | Observe the current terminal state without mutating it |
| `close` | Close the session |

## Why It Matters

This tool is what makes the Terminus-family loops possible without teaching
`agent-core` or `agent-runtime` about a special terminal runtime in their
public API. It is the clearest example of a
tool becoming a long-lived execution surface rather than a one-shot function.
