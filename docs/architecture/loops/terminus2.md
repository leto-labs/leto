# Terminus2 Loop

`Terminus2Loop` is the Harbor-inspired terminal-first loop.

## Core Ideas

| Idea | Current behavior |
| --- | --- |
| terminal-first execution | centers the `terminal_session` tool instead of ordinary one-shot tools |
| structured text planning | prompts for parsed command plans instead of native provider tool calls |
| bounded terminal observation | polls and captures terminal state incrementally |
| proactive summarization | can compact history into a handoff prompt when context pressure rises |
| explicit completion confirmation | asks the model to confirm before marking the task complete |

## Why It Is Different

This loop is not just a more robust `simple` loop. It assumes benchmark-style
terminal tasks, persistent shell state, parse-and-reprompt recovery, and a much
tighter coupling between planning and terminal observation.
