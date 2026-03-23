# Simple Loop

`SimpleLoop` is the baseline `AgentLoop` implementation.

## Execution Model

| Step | Behavior |
| --- | --- |
| 1 | Insert the configured system prompt if present |
| 2 | Call the provider with the current message history and tool definitions |
| 3 | Stream token deltas as `Event::Token` |
| 4 | Collect native provider tool calls |
| 5 | Execute each tool sequentially |
| 6 | Append tool results as tool messages and re-infer until no more tool calls remain |

## Characteristics

| Concern | Current behavior |
| --- | --- |
| Retry behavior | minimal |
| Compaction | none |
| Doom-loop handling | none |
| Terminal specialization | none |

## Use It When

You want the clearest possible baseline for how the shared provider/tool/event
contract works without additional loop policy layered on top.
